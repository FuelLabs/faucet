use fuel_core::chain_config::{
    ChainConfig, CoinConfig, CoinConfigGenerator, SnapshotReader, StateConfig,
};
use fuel_core::service::config::Trigger;
use fuel_core::service::{Config as NodeConfig, FuelService};

use fuel_core_client::client::pagination::{PageDirection, PaginationRequest};
use fuel_crypto::SecretKey;
use fuel_faucet::config::Config;
use fuel_faucet::models::DispenseInfoResponse;
use fuel_faucet::{start_server, Clock};
use fuel_tx::ConsensusParameters;
use fuel_types::Address;
use fuels_accounts::provider::Provider;
use fuels_accounts::wallet::WalletUnlocked;
use fuels_core::types::bech32::Bech32Address;
use fuels_core::types::transaction::TransactionType;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use secrecy::Secret;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::usize;

#[derive(Debug, Clone)]
struct MockClock {
    timer: Arc<Mutex<u64>>,
}

impl Clock for MockClock {
    fn now(&self) -> u64 {
        *self.timer.lock().unwrap()
    }
}

impl MockClock {
    pub fn new() -> Self {
        Self {
            timer: Arc::new(Mutex::new(0)),
        }
    }

    pub fn advance(&self, increment: u64) {
        *self.timer.lock().as_deref_mut().unwrap() += increment
    }
}

struct TestContext {
    #[allow(dead_code)]
    fuel_node: FuelService,
    faucet_config: Config,
    provider: Provider,
    addr: SocketAddr,
    clock: MockClock,
}
impl TestContext {
    async fn new(rng: &mut StdRng) -> Self {
        let dispense_amount = 2000000;
        let secret_key: SecretKey = SecretKey::random(rng);
        let wallet = WalletUnlocked::new_from_private_key(secret_key, None);
        let base_asset_id = [1; 32].into();

        let mut generator = CoinConfigGenerator::new();
        let coins: Vec<_> = (0..10000)
            .map(|_| CoinConfig {
                owner: wallet.address().into(),
                amount: dispense_amount - 1,
                asset_id: base_asset_id,
                ..generator.generate()
            })
            .collect();

        let state_config = StateConfig {
            coins,
            ..Default::default()
        };

        let mut consensus_parameters = ConsensusParameters::default();
        consensus_parameters.set_fee_params(
            // Values from the testnet
            fuel_tx::FeeParameters::default()
                .with_gas_price_factor(92)
                .with_gas_per_byte(63),
        );
        consensus_parameters.set_base_asset_id(base_asset_id);

        let chain_config = ChainConfig {
            consensus_parameters,
            ..ChainConfig::local_testnet()
        };

        let snapshot_reader = SnapshotReader::new_in_memory(chain_config, state_config);

        let mut config = NodeConfig {
            block_production: Trigger::Interval {
                block_time: Duration::from_secs(3),
            },
            utxo_validation: true,
            static_gas_price: 20,
            snapshot_reader,
            ..NodeConfig::local_node()
        };
        config.txpool.max_depth = 32;

        // start node
        let fuel_node = FuelService::new_node(config).await.unwrap();

        // setup provider
        let provider = Provider::connect(&fuel_node.bound_address.to_string())
            .await
            .unwrap();

        // start faucet
        let faucet_config = Config {
            service_port: 0,
            node_url: format!("http://{}", fuel_node.bound_address),
            wallet_secret_key: Some(Secret::new(format!("{secret_key:x}"))),
            dispense_amount,
            number_of_retries: 1,
            ..Default::default()
        };

        let clock = MockClock::new();
        let (addr, _) = start_server(faucet_config.clone(), clock.clone()).await;

        Self {
            fuel_node,
            faucet_config,
            provider,
            addr,
            clock,
        }
    }
}

#[tokio::test]
async fn can_start_server() {
    let context = TestContext::new(&mut StdRng::seed_from_u64(42)).await;
    let addr = context.addr;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{addr}/dispense"))
        .send()
        .await
        .unwrap()
        .json::<DispenseInfoResponse>()
        .await
        .expect("Invalid response body");

    assert_eq!(response.amount, context.faucet_config.dispense_amount);
    assert_eq!(
        response.asset_id,
        context
            .provider
            .consensus_parameters()
            .base_asset_id()
            .to_string()
    );
}

#[tokio::test]
async fn dispense_sends_coins_to_valid_address_hex_address() {
    let mut rng = StdRng::seed_from_u64(42);
    let recipient_address: Address = rng.gen();

    _dispense_sends_coins_to_valid_address(
        rng,
        recipient_address.into(),
        format!("{:#x}", &recipient_address),
    )
    .await
}

#[tokio::test]
async fn dispense_sends_coins_to_valid_address_non_hex() {
    let mut rng = StdRng::seed_from_u64(42);
    let recipient_address: Address = rng.gen();

    _dispense_sends_coins_to_valid_address(
        rng,
        recipient_address.into(),
        format!("{}", &recipient_address),
    )
    .await
}

async fn _dispense_sends_coins_to_valid_address(
    mut rng: StdRng,
    recipient_address: Bech32Address,
    recipient_address_str: String,
) {
    let context = TestContext::new(&mut rng).await;
    let addr = context.addr;
    let client = reqwest::Client::new();

    client
        .post(format!("http://{addr}/dispense"))
        .json(&json!({
            "captcha": "",
            "address": recipient_address_str,
        }))
        .send()
        .await
        .unwrap();

    let test_balance: u64 = context
        .provider
        .get_coins(
            &recipient_address,
            *context.provider.consensus_parameters().base_asset_id(),
        )
        .await
        .unwrap()
        .iter()
        .map(|coin| coin.amount)
        .sum();

    assert!(test_balance >= context.faucet_config.dispense_amount);
}

fn generate_recipient_addresses(count: usize, rng: &mut StdRng) -> Vec<String> {
    let recipient_addresses: Vec<Address> =
        std::iter::repeat_with(|| rng.gen()).take(count).collect();
    recipient_addresses
        .iter()
        .map(|addr| format!("{}", addr))
        .collect()
}

#[tokio::test]
async fn many_concurrent_requests() {
    let mut rng = StdRng::seed_from_u64(42);

    const COUNT: usize = 128;
    let recipient_addresses_str = generate_recipient_addresses(COUNT, &mut rng);
    let context = TestContext::new(&mut rng).await;
    let addr = context.addr;

    let mut queries = vec![];
    for recipient in recipient_addresses_str {
        let recipient = recipient.clone();
        queries.push(async move {
            let client = reqwest::Client::new();
            client
                .post(format!("http://{addr}/dispense"))
                .json(&json!({
                    "captcha": "",
                    "address": recipient,
                }))
                .send()
                .await
        });
    }
    let mut queries = FuturesUnordered::from_iter(queries);
    let mut success = 0;
    while let Some(query) = queries.next().await {
        let response = query.expect("Query should be successful");
        assert_eq!(
            response.status(),
            reqwest::StatusCode::CREATED,
            "{success}/{COUNT}: {:?}",
            response.bytes().await
        );
        success += 1;
    }

    let txs = context
        .provider
        .get_transactions(PaginationRequest {
            cursor: None,
            results: 500,
            direction: PageDirection::Forward,
        })
        .await
        .unwrap()
        .results
        .into_iter()
        .filter(|tx| !matches!(tx.transaction, TransactionType::Mint(_)))
        .collect::<Vec<_>>();

    assert_eq!(COUNT, txs.len());
}

#[tokio::test]
async fn dispense_once_per_day() {
    let mut rng = StdRng::seed_from_u64(42);
    let recipient_address: Address = rng.gen();
    let recipient_address_str = format!("{}", &recipient_address);
    let context = TestContext::new(&mut rng).await;
    let addr = context.addr;

    let dispense_interval = 24 * 60 * 60;
    let time_increment = dispense_interval / 6;

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/dispense"))
        .json(&json!({
            "captcha": "",
            "address": recipient_address_str.clone(),
        }))
        .send()
        .await
        .expect("First dispensing request should be successful");

    assert_eq!(response.status(), reqwest::StatusCode::CREATED);

    for _ in 0..5 {
        context.clock.advance(time_increment);

        let response = reqwest::Client::new()
            .post(format!("http://{addr}/dispense"))
            .json(&json!({
                "captcha": "",
                "address": recipient_address_str.clone(),
            }))
            .send()
            .await
            .expect("Subsequent dispensing requests should be successfully sent");

        assert_eq!(response.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
    }

    context.clock.advance(time_increment + 1);
    let response = reqwest::Client::new()
        .post(format!("http://{addr}/dispense"))
        .json(&json!({
            "captcha": "",
            "address": recipient_address_str.clone(),
        }))
        .send()
        .await
        .expect("Dispensing requests after the interval should be successful");

    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
}

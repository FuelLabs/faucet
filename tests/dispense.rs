use fuel_core::chain_config::{ChainConfig, CoinConfig, StateConfig};
use fuel_core::service::config::Trigger;
use fuel_core::service::{Config as NodeConfig, FuelService};

use fuel_core_client::client::pagination::{PageDirection, PaginationRequest};
use fuel_faucet::config::Config;

use fuel_faucet::models::DispenseInfoResponse;
use fuel_faucet::{start_server, Clock, THE_BIGGEST_AMOUNT};

use fuel_tx::{ConsensusParameters, FeeParameters};
use fuel_types::{Address, AssetId};
use fuels_accounts::fuel_crypto::SecretKey;
use fuels_accounts::provider::Provider;
use fuels_accounts::wallet::WalletUnlocked;
use fuels_core::types::bech32::Bech32Address;
use fuels_core::types::transaction::TransactionType;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use secrecy::Secret;
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
    async fn new(mut rng: StdRng) -> Self {
        let dispense_amount = rng.gen_range(1..10000u64);
        let secret_key: SecretKey = SecretKey::random(&mut rng);
        let wallet = WalletUnlocked::new_from_private_key(secret_key, None);

        let mut coins: Vec<_> = (0..10000)
            .map(|_| {
                // dust
                CoinConfig {
                    tx_id: None,
                    output_index: None,
                    maturity: None,
                    tx_pointer_block_height: None,
                    tx_pointer_tx_idx: None,
                    owner: wallet.address().into(),
                    amount: THE_BIGGEST_AMOUNT - 1,
                    asset_id: rng.gen(),
                }
            })
            .collect();
        // main coin
        coins.push(CoinConfig {
            tx_id: None,
            output_index: None,
            maturity: None,
            tx_pointer_block_height: None,
            tx_pointer_tx_idx: None,
            owner: wallet.address().into(),
            amount: 1 << 50,
            asset_id: Default::default(),
        });

        // start node
        let fuel_node = FuelService::new_node(NodeConfig {
            chain_conf: ChainConfig {
                initial_state: Some(StateConfig {
                    coins: Some(coins),
                    contracts: None,
                    height: None,
                    messages: None,
                }),
                consensus_parameters: ConsensusParameters {
                    fee_params: FeeParameters {
                        gas_price_factor: 1,
                        ..Default::default()
                    },
                    ..ConsensusParameters::default()
                },
                ..ChainConfig::local_testnet()
            },
            block_production: Trigger::Interval {
                block_time: Duration::from_secs(3),
            },
            txpool: fuel_core_txpool::Config {
                min_gas_price: 1,
                ..Default::default()
            },
            utxo_validation: true,
            ..NodeConfig::local_node()
        })
        .await
        .unwrap();

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
            dispense_asset_id: AssetId::default(),
            min_gas_price: 1,
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
    let context = TestContext::new(StdRng::seed_from_u64(42)).await;
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
        context.faucet_config.dispense_asset_id.to_string()
    );
}

#[tokio::test]
async fn dispense_sends_coins_to_valid_address_hex_address() {
    let mut rng = StdRng::seed_from_u64(42);
    let recipient_address: Address = rng.gen();

    dispense_sends_coins_to_valid_address(rng, recipient_address.into()).await
}

#[tokio::test]
async fn dispense_sends_coins_to_valid_address_non_hex() {
    let mut rng = StdRng::seed_from_u64(42);
    let recipient_address: Address = rng.gen();

    dispense_sends_coins_to_valid_address(rng, recipient_address.into()).await
}

async fn dispense_sends_coins_to_valid_address(rng: StdRng, recipient_address: Bech32Address) {
    let context = TestContext::new(rng).await;
    let addr = context.addr;
    let client = reqwest::Client::new();

    client
        .post(format!("http://{addr}/api/dispense"))
        .send()
        .await
        .unwrap();

    let test_balance: u64 = context
        .provider
        .get_coins(&recipient_address, context.faucet_config.dispense_asset_id)
        .await
        .unwrap()
        .iter()
        .map(|coin| coin.amount)
        .sum();

    assert_eq!(test_balance, context.faucet_config.dispense_amount);
}

#[tokio::test]
async fn many_concurrent_requests() {
    let rng = StdRng::seed_from_u64(42);
    const COUNT: usize = 30;
    let context = TestContext::new(rng).await;

    let txs = context
        .provider
        .get_transactions(PaginationRequest {
            cursor: None,
            results: 1000,
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
    let context = TestContext::new(rng).await;
    let addr = context.addr;

    let dispense_interval = 24 * 60 * 60;
    let time_increment = dispense_interval / 6;

    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{addr}/api/dispense"))
        .send()
        .await
        .expect("First dispensing request should be successful");

    assert_eq!(response.status(), reqwest::StatusCode::CREATED);

    for _ in 0..5 {
        context.clock.advance(time_increment);

        let response = reqwest::Client::new()
            .post(format!("http://{addr}/api/dispense"))
            .send()
            .await
            .expect("Subsequent dispensing requests should be successfully sent");

        assert_eq!(response.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
    }

    context.clock.advance(time_increment + 1);
    let response = reqwest::Client::new()
        .post(format!("http://{addr}/api/dispense"))
        .send()
        .await
        .expect("Dispensing requests after the interval should be successful");

    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
}

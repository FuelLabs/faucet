use fuel_core::chain_config::{ChainConfig, CoinConfig, StateConfig};
use fuel_core::service::config::Trigger;
use fuel_core::service::{Config as NodeConfig, FuelService};

use fuel_faucet::config::Config;
use fuel_faucet::models::DispenseInfoResponse;
use fuel_faucet::{start_server, THE_BIGGEST_AMOUNT};
use fuel_types::{Address, AssetId};
use fuels_accounts::fuel_crypto::SecretKey;
use fuels_accounts::provider::Provider;
use fuels_accounts::wallet::WalletUnlocked;
use fuels_core::types::bech32::Bech32Address;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use secrecy::Secret;
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;

struct TestContext {
    #[allow(dead_code)]
    fuel_node: FuelService,
    faucet_config: Config,
    provider: Provider,
    addr: SocketAddr,
}
impl TestContext {
    async fn new(mut rng: StdRng) -> Self {
        let dispense_amount = rng.gen_range(1..10000u64);
        let secret_key: SecretKey = rng.gen();
        let wallet = WalletUnlocked::new_from_private_key(
            secret_key,
            Some(
                Provider::connect(&SocketAddr::from(([0, 0, 0, 0], 0)).to_string())
                    .await
                    .unwrap(),
            ),
        );

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
                    asset_id: Default::default(),
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
        let (addr, _) = start_server(faucet_config.clone()).await;

        Self {
            fuel_node,
            faucet_config,
            provider,
            addr,
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
    rng: StdRng,
    recipient_address: Bech32Address,
    recipient_address_str: String,
) {
    let context = TestContext::new(rng).await;
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
    let mut rng = StdRng::seed_from_u64(42);
    let recipient_address: Address = rng.gen();
    let recipient_address_str = format!("{}", &recipient_address);
    let context = TestContext::new(rng).await;
    let addr = context.addr;

    let mut queries = vec![];
    // The same as `DEFAULT_MAX_DISPENSES_PER_MINUTE`.
    const COUNT: usize = 20;
    for _ in 0..COUNT {
        let recipient_address_str = recipient_address_str.clone();
        queries.push(async move {
            let client = reqwest::Client::new();
            client
                .post(format!("http://{addr}/dispense"))
                .json(&json!({
                    "captcha": "",
                    "address": recipient_address_str,
                }))
                .send()
                .await
        });
    }

    let queries = futures::future::join_all(queries).await;

    for query in queries {
        query.expect("Query should be successful");
    }

    let test_balance: u64 = context
        .provider
        .get_coins(
            &recipient_address.into(),
            context.faucet_config.dispense_asset_id,
        )
        .await
        .unwrap()
        .iter()
        .map(|coin| coin.amount)
        .sum();
    assert_eq!(
        test_balance,
        COUNT as u64 * context.faucet_config.dispense_amount
    );
}

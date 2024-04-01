use axum::async_trait;
use fuel_core::chain_config::{ChainConfig, CoinConfig, StateConfig};
use fuel_core::service::config::Trigger;
use fuel_core::service::{Config as NodeConfig, FuelService};

use fuel_core_client::client::pagination::{PageDirection, PaginationRequest};
use fuel_faucet::auth::{AuthError, AuthHandler};
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
use serde_json::json;
use std::collections::HashSet;
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

#[derive(Debug, Clone)]
struct MockAuthHandler {
    user_ids: Arc<Mutex<HashSet<String>>>,
}

impl MockAuthHandler {
    pub fn new() -> Self {
        Self {
            user_ids: Default::default(),
        }
    }

    pub fn register_user(&self, user_id: String) {
        self.user_ids.lock().as_deref_mut().unwrap().insert(user_id);
    }

    pub fn is_registered(&self, user_id: &str) -> bool {
        self.user_ids.lock().as_deref().unwrap().contains(user_id)
    }
}

#[async_trait]
impl AuthHandler for MockAuthHandler {
    async fn get_user_session(&self, user_id: &str) -> Result<String, AuthError> {
        if self.is_registered(user_id) {
            Ok(user_id.to_string())
        } else {
            Err(AuthError::new("User needs to be registered"))
        }
    }
}

struct TestContext {
    #[allow(dead_code)]
    fuel_node: FuelService,
    faucet_config: Config,
    provider: Provider,
    addr: SocketAddr,
    auth_handler: MockAuthHandler,
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
        let auth_handler = MockAuthHandler::new();
        let (addr, _) =
            start_server(faucet_config.clone(), clock.clone(), auth_handler.clone()).await;

        Self {
            fuel_node,
            faucet_config,
            provider,
            addr,
            auth_handler,
            clock,
        }
    }
}

struct DispenseRequest {
    client: reqwest::Client,
    recipient_address: String,
    addr: SocketAddr,
}
impl DispenseRequest {
    fn for_recipient(recipient_address: String, context: &TestContext) -> Self {
        context
            .auth_handler
            .register_user(recipient_address.clone());

        let client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .build()
            .unwrap();

        Self {
            client,
            recipient_address,
            addr: context.addr,
        }
    }

    async fn send(&self) -> reqwest::Result<reqwest::Response> {
        let addr = self.addr;
        self.client
            .post(format!("http://{addr}/api/session/validate"))
            .json(&json!({
                "value": self.recipient_address,
            }))
            .send()
            .await
            .unwrap();

        self.client
            .post(format!("http://{addr}/api/dispense"))
            .json(&json!({
                "address": self.recipient_address,
            }))
            .send()
            .await
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

    dispense_sends_coins_to_valid_address(
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

    dispense_sends_coins_to_valid_address(
        rng,
        recipient_address.into(),
        format!("{}", &recipient_address),
    )
    .await
}

async fn dispense_sends_coins_to_valid_address(
    rng: StdRng,
    recipient_address: Bech32Address,
    recipient_address_str: String,
) {
    let context = TestContext::new(rng).await;

    context
        .auth_handler
        .register_user(recipient_address_str.clone());

    DispenseRequest::for_recipient(recipient_address_str, &context)
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
    const COUNT: usize = 30;
    let recipient_addresses_str = generate_recipient_addresses(COUNT, &mut rng);
    let context = TestContext::new(rng).await;

    let mut queries = vec![];
    for recipient in recipient_addresses_str {
        queries.push(async {
            context.auth_handler.register_user(recipient.clone());
            DispenseRequest::for_recipient(recipient, &context)
                .send()
                .await
        });
    }
    let queries = futures::future::join_all(queries).await;
    for query in queries {
        query.expect("Query should be successful");
    }

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

    let dispense_interval = 24 * 60 * 60;
    let time_increment = dispense_interval / 6;

    context
        .auth_handler
        .register_user(recipient_address_str.clone());
    let response = DispenseRequest::for_recipient(recipient_address_str.clone(), &context)
        .send()
        .await
        .expect("First dispensing request should be successful");

    assert_eq!(response.status(), reqwest::StatusCode::CREATED);

    for _ in 0..5 {
        context.clock.advance(time_increment);

        let response = DispenseRequest::for_recipient(recipient_address_str.clone(), &context)
            .send()
            .await
            .expect("Subsequent dispensing requests should be successfully sent");

        assert_eq!(response.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
    }

    context.clock.advance(time_increment + 1);
    let response = DispenseRequest::for_recipient(recipient_address_str.clone(), &context)
        .send()
        .await
        .expect("Dispensing requests after the interval should be successful");

    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
}

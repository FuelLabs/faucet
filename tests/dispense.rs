use fuel_core::chain_config::{ChainConfig, CoinConfig, StateConfig};
use fuel_core::service::{Config as NodeConfig, FuelService};
use fuel_crypto::SecretKey;
use fuel_faucet::config::Config;
use fuel_faucet::start_server;
use fuel_faucet::structures::DispenseInfoResponse;
use fuel_types::{Address, AssetId};
use fuels_signers::provider::Provider;
use fuels_signers::wallet::Wallet;
use fuels_signers::Signer;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use secrecy::Secret;
use serde_json::json;
use std::net::SocketAddr;

#[tokio::test]
async fn dispense_sends_coins_to_valid_address() {
    let mut rng = StdRng::seed_from_u64(42);
    let dispense_amount = rng.gen_range(1..10000u64);
    let recipient_address: Address = rng.gen();
    let secret_key: SecretKey = rng.gen();
    let wallet = Wallet::new_from_private_key(
        secret_key,
        Provider::connect(SocketAddr::from(([0, 0, 0, 0], 0)))
            .await
            .unwrap(),
    );

    // start node
    let fuel_node = FuelService::new_node(NodeConfig {
        chain_conf: ChainConfig {
            initial_state: Some(StateConfig {
                coins: Some(vec![CoinConfig {
                    tx_id: None,
                    output_index: None,
                    block_created: None,
                    maturity: None,
                    owner: wallet.address(),
                    amount: 1 << 50,
                    asset_id: Default::default(),
                }]),
                contracts: None,
                height: None,
            }),
            ..ChainConfig::local_testnet()
        },
        tx_pool_config: fuel_txpool::Config {
            min_gas_price: 1,
            min_byte_price: 1,
            ..Default::default()
        },
        ..NodeConfig::local_node()
    })
    .await
    .unwrap();

    // setup provider
    let provider = Provider::connect(fuel_node.bound_address).await.unwrap();

    // start faucet
    let faucet_config = Config {
        service_port: 0,
        node_url: format!("http://{}", fuel_node.bound_address),
        wallet_secret_key: Some(Secret::new(format!("{:x}", secret_key))),
        dispense_amount,
        dispense_asset_id: AssetId::default(),
        min_gas_price: 1,
        min_byte_price: 1,
        ..Default::default()
    };
    let (addr, _) = start_server(faucet_config.clone()).await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/dispense", addr))
        .send()
        .await
        .unwrap()
        .json::<DispenseInfoResponse>()
        .await
        .expect("Invalid response body");

    assert_eq!(response.amount, faucet_config.dispense_amount);
    assert_eq!(
        response.asset_id,
        faucet_config.dispense_asset_id.to_string()
    );

    client
        .post(format!("http://{}/dispense", addr))
        .json(&json!({
            "captcha": "",
            "address": format!("{:#x}", &recipient_address)
        }))
        .send()
        .await
        .unwrap();

    let test_balance: u64 = provider
        .get_coins(&recipient_address)
        .await
        .unwrap()
        .iter()
        .map(|coin| coin.amount.0)
        .sum();

    assert_eq!(test_balance, faucet_config.dispense_amount);
}

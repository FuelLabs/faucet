use crate::{
    config::Config,
    constants::{MAX_CONCURRENT_REQUESTS, WALLET_SECRET_DEV_KEY},
    routes::health,
};
use anyhow::anyhow;
use axum::{
    error_handling::HandleErrorLayer,
    http::{header::CACHE_CONTROL, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    BoxError, Extension, Json, Router,
};
use fuel_core_client::client::FuelClient;
use fuel_tx::{ConsensusParameters, UtxoId};
use fuel_types::Address;
use fuels_signers::{provider::Provider, wallet::WalletUnlocked, Signer};
use fuels_types::node_info::NodeInfo;
use secrecy::{ExposeSecret, Secret};
use serde_json::json;
use std::collections::BTreeSet;
use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::Duration,
};
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::info;

pub mod config;
pub mod models;

mod constants;
mod recaptcha;
mod routes;

#[derive(Debug)]
pub struct NetworkConfig {
    pub consensus_parameters: ConsensusParameters,
    pub node_info: NodeInfo,
}

#[derive(Debug, Copy, Clone)]
pub struct CoinOutput {
    utxo_id: UtxoId,
    owner: Address,
    amount: u64,
}

#[derive(Debug)]
pub struct FaucetState {
    // Gas prices create the ordering for transactions.
    pub gas_prices: BTreeSet<u64>,
    pub last_output: Option<CoinOutput>,
}

impl FaucetState {
    pub fn new(min_gas_price: u64, node_info: &NodeInfo) -> Self {
        let gas_prices = (0u64..node_info.max_depth).map(|gas_price| gas_price + min_gas_price);
        Self {
            gas_prices: BTreeSet::from_iter(gas_prices),
            last_output: None,
        }
    }
}

pub type SharedFaucetState = Arc<tokio::sync::Mutex<FaucetState>>;
pub type SharedWallet = Arc<WalletUnlocked>;
pub type SharedConfig = Arc<Config>;
pub type SharedNetworkConfig = Arc<NetworkConfig>;

pub async fn start_server(
    service_config: Config,
) -> (SocketAddr, JoinHandle<Result<(), anyhow::Error>>) {
    info!("{:#?}", &service_config);

    // connect to the fuel node
    let client = FuelClient::new(service_config.node_url.clone())
        .expect("unable to connect to the fuel node api");
    let provider = Provider::new(client);
    let node_info = provider
        .node_info()
        .await
        .expect("unable to get `node_info`");
    let consensus_parameters = provider
        .chain_info()
        .await
        .expect("unable to get `chain_info`")
        .consensus_parameters;

    let network_config = NetworkConfig {
        consensus_parameters,
        node_info,
    };

    // setup wallet
    let secret = service_config
        .wallet_secret_key
        .clone()
        .unwrap_or_else(|| Secret::new(WALLET_SECRET_DEV_KEY.to_string()));
    let wallet = WalletUnlocked::new_from_private_key(
        secret
            .expose_secret()
            .parse()
            .expect("Unable to load secret key"),
        Some(provider),
    );

    let balance = wallet
        .get_coins(service_config.dispense_asset_id)
        .await
        .expect("Failed to fetch initial balance from fuel core")
        .into_iter()
        .filter_map(|coin| match coin.status {
            fuels_types::coin::CoinStatus::Unspent => Some(coin.amount),
            _ => None,
        })
        .sum::<u64>();
    info!("Faucet Account: {:#x}", Address::from(wallet.address()));
    info!("Faucet Balance: {}", balance);

    // setup routes
    let app = Router::new()
        .route(
            "/",
            get(routes::main).layer(SetResponseHeaderLayer::<_>::overriding(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600, immutable"),
            )),
        )
        .route("/health", get(health))
        .route("/dispense", get(routes::dispense_info))
        .route(
            "/dispense",
            post(routes::dispense_tokens).route_layer(
                // Apply rate limiting specifically on the dispense endpoint, and
                // only allow a single instance at a time to avoid race conditions
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(handle_error))
                    .buffer(MAX_CONCURRENT_REQUESTS)
                    .rate_limit(
                        service_config.max_dispenses_per_minute,
                        Duration::from_secs(60),
                    )
                    .concurrency_limit(network_config.node_info.max_depth as usize)
                    .into_inner(),
            ),
        )
        .layer(
            ServiceBuilder::new()
                // Handle errors from middleware
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(MAX_CONCURRENT_REQUESTS)
                .timeout(Duration::from_secs(60))
                .layer(TraceLayer::new_for_http())
                .layer(Extension(Arc::new(wallet)))
                .layer(Extension(Arc::new(tokio::sync::Mutex::new(
                    FaucetState::new(service_config.min_gas_price, &network_config.node_info),
                ))))
                .layer(Extension(Arc::new(service_config.clone())))
                .layer(Extension(Arc::new(network_config)))
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
                .into_inner(),
        );

    // run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], service_config.service_port));
    let listener = TcpListener::bind(addr).unwrap();
    let bound_addr = listener.local_addr().unwrap();
    info!("listening on {}", bound_addr);
    (
        bound_addr,
        tokio::spawn(async move {
            axum::Server::from_tcp(listener)
                .unwrap()
                .serve(app.into_make_service())
                .await
                .map_err(|e| anyhow!(e))
        }),
    )
}

async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (
            StatusCode::REQUEST_TIMEOUT,
            Json(json!({
                "error": "request timed out"
            })),
        );
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "service is overloaded, try again later"
            })),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "error": format!("Unhandled internal error: {error}")
        })),
    )
}

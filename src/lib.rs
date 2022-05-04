use crate::routes::health;
use crate::{config::Config, constants::WALLET_SECRET_DEV_KEY};
use anyhow::anyhow;
use axum::{
    http::header::CACHE_CONTROL,
    http::HeaderValue,
    routing::{get, post},
    Extension, Router,
};
use fuel_gql_client::client::FuelClient;
use fuels_signers::{provider::Provider, wallet::Wallet};
use secrecy::{ExposeSecret, Secret};
use std::{net::SocketAddr, net::TcpListener, sync::Arc};
use tokio::task::JoinHandle;
use tower_http::{
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
};
use tracing::info;
use tracing_subscriber::filter::EnvFilter;

pub mod config;
mod constants;
mod recaptcha;
mod routes;

pub type SharedWallet = Arc<Wallet>;
pub type SharedConfig = Arc<Config>;

pub async fn start_server(
    service_config: Config,
) -> (SocketAddr, JoinHandle<Result<(), anyhow::Error>>) {
    init_logger(&service_config);
    tracing::info!("{:#?}", &service_config);

    // connect to the fuel node
    let client = FuelClient::new(service_config.node_url.clone())
        .expect("unable to connect to the fuel node api");
    let provider = Provider::new(client);
    // setup wallet
    let secret = service_config
        .wallet_secret_key
        .clone()
        .unwrap_or_else(|| Secret::new(WALLET_SECRET_DEV_KEY.to_string()));
    let wallet = Wallet::new_from_private_key(
        secret
            .expose_secret()
            .parse()
            .expect("Unable to load secret key"),
        provider,
    )
    .unwrap();

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
        .route("/dispense", post(routes::dispense_tokens))
        .layer(Extension(Arc::new(wallet)))
        .layer(Extension(Arc::new(service_config.clone())))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any));

    // run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], service_config.service_port));
    let listener = TcpListener::bind(&addr).unwrap();
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

fn init_logger(config: &Config) {
    let filter = if !config.log_filter.is_empty() {
        EnvFilter::try_from_default_env().expect("Invalid `RUST_LOG` provided")
    } else {
        EnvFilter::new("info")
    };

    let sub = tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(std::io::stderr)
        .with_env_filter(filter);

    if config.human_logging {
        // use pretty logs
        sub.init();
    } else {
        // use machine parseable structured logs
        sub
            // disable terminal colors
            .with_ansi(false)
            // use json
            .json()
            .init();
    }
}

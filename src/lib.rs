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
use fuel_gql_client::client::FuelClient;
use fuels_signers::{provider::Provider, wallet::Wallet};
use secrecy::{ExposeSecret, Secret};
use serde_json::json;
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
        .route(
            "/dispense",
            post(routes::dispense_tokens).route_layer(
                // apply rate limiting specifically on the dispense endpoint
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(handle_error))
                    .buffer(MAX_CONCURRENT_REQUESTS)
                    .rate_limit(
                        service_config.max_dispenses_per_minute,
                        Duration::from_secs(60),
                    )
                    .into_inner(),
            ),
        )
        .layer(
            ServiceBuilder::new()
                // Handle errors from middleware
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(MAX_CONCURRENT_REQUESTS)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(Extension(Arc::new(wallet)))
                .layer(Extension(Arc::new(service_config.clone())))
                .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any))
                .into_inner(),
        );

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
            "error": format!("Unhandled internal error: {}", error)
        })),
    )
}

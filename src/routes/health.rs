use std::time::{SystemTime, UNIX_EPOCH};

use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
use serde_json::json;

use crate::SharedWallet;

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

#[tracing::instrument(skip(wallet))]
pub async fn handler(Extension(wallet): Extension<SharedWallet>) -> impl IntoResponse {
    // ping client for health
    let client = wallet
        .provider()
        .expect("client provider")
        .healthy()
        .await
        .unwrap_or(false);

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let status = if client {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    (
        status,
        Json(json!({
            "up": true,
            "uptime": time - *START_TIME,
            "fuel-core" : client,
        })),
    )
}

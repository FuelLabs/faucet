use crate::{models::*, recaptcha, SharedConfig, SharedWallet};
use axum::{
    response::{Html, IntoResponse, Response},
    Extension, Json,
};
use fuel_types::Address;
use fuels_core::parameters::TxParameters;
use handlebars::Handlebars;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde_json::json;
use std::{
    collections::BTreeMap,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};

lazy_static::lazy_static! {
    static ref PAGE: String = {
        let template = include_str!(concat!(env!("OUT_DIR"), "/index.html"));

        // sub in values
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("index", template).unwrap();
        let mut data = BTreeMap::new();
        data.insert("page_title", "Fuel Faucet");
        handlebars.render("index", &data).unwrap()
    };

    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

pub async fn main() -> Html<&'static str> {
    Html(&PAGE)
}

pub async fn health() -> Json<serde_json::Value> {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    Json(json!({ "up": true, "uptime": time - *START_TIME }))
}

impl IntoResponse for DispenseResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

impl IntoResponse for DispenseError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": self.error
            })),
        )
            .into_response()
    }
}

impl IntoResponse for DispenseInfoResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[tracing::instrument(skip(wallet, config))]
pub async fn dispense_tokens(
    Json(input): Json<DispenseInput>,
    Extension(wallet): Extension<SharedWallet>,
    Extension(config): Extension<SharedConfig>,
) -> Result<DispenseResponse, DispenseError> {
    // parse deposit address
    let address = Address::from_str(input.address.as_str()).map_err(|_| DispenseError {
        status: StatusCode::BAD_REQUEST,
        error: "invalid address".to_string(),
    })?;

    // verify captcha
    if let Some(s) = config.captcha_secret.clone() {
        recaptcha::verify(s.expose_secret(), input.captcha.as_str(), None)
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                DispenseError {
                    error: "captcha failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            })?;
    }

    // transfer tokens
    wallet
        .transfer(
            &address,
            config.dispense_amount,
            config.dispense_asset_id,
            TxParameters {
                gas_price: config.min_gas_price,
                byte_price: config.min_byte_price,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| {
            error!("failed to transfer: {}", e);
            DispenseError {
                error: "Failed to transfer".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    info!(
        "dispensed {} tokens to {:#x}",
        config.dispense_amount, &address
    );

    Ok(DispenseResponse {
        status: "Success".to_string(),
        tokens: config.dispense_amount,
    })
}

#[tracing::instrument(skip(config))]
pub async fn dispense_info(
    Extension(config): Extension<SharedConfig>,
) -> Result<DispenseInfoResponse, DispenseError> {
    Ok(DispenseInfoResponse {
        amount: config.dispense_amount,
        asset_id: config.dispense_asset_id.to_string(),
    })
}

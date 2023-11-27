use crate::{
    models::*, recaptcha, CoinOutput, SharedConfig, SharedFaucetState, SharedNetworkConfig,
    SharedWallet,
};
use axum::{
    response::{Html, IntoResponse, Response},
    Extension, Json,
};

use fuel_tx::{TransactionFee, UniqueIdentifier, UtxoId};
use fuel_types::{Address, AssetId};
use fuels_accounts::{Account, Signer};
use fuels_core::types::{
    bech32::Bech32Address,
    coin::{Coin, CoinStatus},
    coin_type::CoinType,
    transaction::TxParameters,
};
use fuels_core::types::{
    input::Input,
    transaction_builders::{ScriptTransactionBuilder, TransactionBuilder},
};
use handlebars::Handlebars;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde_json::json;
use std::{
    collections::BTreeMap,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use std::{collections::HashMap, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info};

// The amount to fetch the biggest input of the faucet.
pub const THE_BIGGEST_AMOUNT: u64 = u32::MAX as u64;

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

lazy_static::lazy_static! {
    static ref LAST_DISPENSED: Mutex<HashMap<Address, u64>> = Mutex::new(HashMap::new());
}

#[memoize::memoize]
pub fn render_page(public_node_url: String, captcha_key: Option<String>) -> String {
    let template = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
    // sub in values
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("index", template)
        .unwrap();
    let mut data = BTreeMap::new();
    data.insert("page_title", "Fuel Faucet");
    data.insert("public_node_url", public_node_url.as_str());
    // if captcha is enabled, add captcha key
    if let Some(captcha_key) = &captcha_key {
        data.insert("captcha_key", captcha_key.as_str());
    }
    // render page
    handlebars.render("index", &data).unwrap()
}

pub async fn main(Extension(config): Extension<SharedConfig>) -> Html<String> {
    let public_node_url = config.public_node_url.clone();
    let captcha_key = config.captcha_key.clone();
    Html(render_page(public_node_url, captcha_key))
}

#[tracing::instrument(skip(wallet))]
pub async fn health(Extension(wallet): Extension<SharedWallet>) -> Response {
    // ping client for health
    let client = wallet
        .provider()
        .expect("client provider")
        .client
        .health()
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
        .into_response()
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
    Extension(state): Extension<SharedFaucetState>,
    Extension(config): Extension<SharedConfig>,
    Extension(network_config): Extension<SharedNetworkConfig>,
) -> Result<DispenseResponse, DispenseError> {
    // parse deposit address
    let address = if let Ok(address) = Address::from_str(input.address.as_str()) {
        Ok(address)
    } else if let Ok(address) = Bech32Address::from_str(input.address.as_str()) {
        Ok(address.into())
    } else {
        return Err(DispenseError {
            status: StatusCode::BAD_REQUEST,
            error: "invalid address".to_string(),
        });
    }?;

    let last_dispensed = LAST_DISPENSED
        .lock()
        .await
        .get(&address)
        .copied()
        .unwrap_or_default();
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check if the account has received assets in the last 24 hours
    if current_time - last_dispensed < 24 * 60 * 60 {
        return Err(DispenseError {
            status: StatusCode::FORBIDDEN,
            error: "Account has already received assets today".to_string(),
        });
    }

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

    let provider = wallet.provider().expect("client provider");

    let mut tx;

    loop {
        let mut guard = state.lock().await;
        let coin_output = if let Some(previous_coin_output) = &guard.last_output {
            *previous_coin_output
        } else {
            provider
                .client
                .coins_to_spend(
                    &wallet.address().into(),
                    vec![(AssetId::BASE, THE_BIGGEST_AMOUNT, None)],
                    None,
                )
                .await
                .map_err(|e| error(format!("Failed to get resources: {}", e)))?
                .into_iter()
                .flatten()
                .filter_map(|resource| match resource {
                    fuel_core_client::client::types::CoinType::Coin(coin) => Some(CoinOutput {
                        utxo_id: coin.utxo_id,
                        owner: coin.owner,
                        amount: coin.amount,
                    }),
                    _ => None,
                })
                .last()
                .expect("The wallet is empty")
        };

        let coin_type = CoinType::Coin(Coin {
            amount: coin_output.amount,
            block_created: 0u32,
            asset_id: config.dispense_asset_id,
            utxo_id: coin_output.utxo_id,
            maturity: 0u32,
            owner: coin_output.owner.into(),
            status: CoinStatus::Unspent,
        });

        let inputs = vec![Input::resource_signed(coin_type, 0)];

        let outputs = wallet.get_asset_outputs_for_amount(
            &address.into(),
            config.dispense_asset_id,
            config.dispense_amount,
        );

        let gas_price = guard.next_gas_price();

        let mut script = ScriptTransactionBuilder::prepare_transfer(
            inputs,
            outputs,
            TxParameters::default().set_gas_price(gas_price),
        )
        .set_gas_limit(0)
        .build()
        .expect("valid script");

        wallet
            .sign_transaction(&mut script)
            .map_err(|e| error(format!("Failed to sign transaction: {}", e)))?;

        let total_fee =
            TransactionFee::checked_from_tx(&network_config.consensus_parameters, &script.tx)
                .expect("Can't overflow with transfer transaction");

        tx = script.into();
        let result = tokio::time::timeout(
            Duration::from_secs(config.timeout),
            provider.client.submit(&tx),
        )
        .await
        .map(|r| r.map_err(|e| error(format!("Failed to submit transaction: {}", e))))
        .map_err(|e| {
            error(format!(
                "Timeout during while submitting transaction: {}",
                e
            ))
        });

        LAST_DISPENSED
            .lock()
            .await
            .insert(address.clone().into(), current_time);

        match result {
            Ok(Ok(_)) => {
                guard.last_output = Some(CoinOutput {
                    utxo_id: UtxoId::new(tx.id(&network_config.consensus_parameters.chain_id), 1),
                    owner: coin_output.owner,
                    amount: coin_output.amount - total_fee.max_fee() - config.dispense_amount,
                });
                break;
            }
            _ => {
                guard.last_output = None;
            }
        };
    }

    tokio::time::timeout(
        Duration::from_secs(config.timeout),
        provider
            .client
            .await_transaction_commit(&tx.id(&network_config.consensus_parameters.chain_id)),
    )
    .await
    .map(|r| r.map_err(|e| error(format!("Failed to submit transaction with error: {}", e))))
    .map_err(|e| {
        error(format!(
            "Got a timeout during transaction submission: {}",
            e
        ))
    })??;

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

fn error(error: String) -> DispenseError {
    error!("{}", error);
    DispenseError {
        error,
        status: StatusCode::INTERNAL_SERVER_ERROR,
    }
}

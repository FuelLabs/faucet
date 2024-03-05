use crate::{
    models::*,
    recaptcha,
    session::Salt,
    CoinOutput, SharedConfig, SharedDispenseTracker, SharedFaucetState, SharedNetworkConfig,
    SharedSessions, SharedWallet,
};
use axum::{
    extract::Query,
    response::{Html, IntoResponse, Response},
    routing::{get_service, MethodRouter},
    Extension, Json,
};

use fuel_core_client::client::FuelClient;
use fuel_tx::UtxoId;
use fuel_types::{Address, AssetId, Bytes32};
use fuels_accounts::{wallet::WalletUnlocked, Account, Signer, ViewOnlyAccount};
use fuels_core::types::transaction::{Transaction, TxPolicies};
use fuels_core::types::transaction_builders::BuildableTransaction;
use fuels_core::types::{
    bech32::Bech32Address,
    coin::{Coin, CoinStatus},
    coin_type::CoinType,
};
use fuels_core::types::{input::Input, transaction_builders::ScriptTransactionBuilder};
use handlebars::Handlebars;
use hex::FromHexError;
use num_bigint::BigUint;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    io,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tower_http::services::ServeFile;
use tracing::{error, info};

// The amount to fetch the biggest input of the faucet.
pub const THE_BIGGEST_AMOUNT: u64 = u32::MAX as u64;

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
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

#[memoize::memoize]
pub fn serve_worker() -> MethodRouter {
    let template = concat!(env!("OUT_DIR"), "/worker.js");

    async fn handle_error(_err: io::Error) -> impl IntoResponse {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not serve worker.js",
        )
    }
    get_service(ServeFile::new(template)).handle_error(handle_error)
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

fn check_and_mark_dispense_limit(
    dispense_tracker: &SharedDispenseTracker,
    address: Address,
    interval: u64,
) -> Result<(), DispenseError> {
    let mut tracker = dispense_tracker.lock().unwrap();
    tracker.evict_expired_entries(interval);

    if tracker.has_tracked(&address) {
        return Err(error(
            "Account has already received assets today".to_string(),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }

    tracker.mark_in_progress(address);
    Ok(())
}

async fn get_coin_output(
    wallet: &WalletUnlocked,
    amount: u64,
) -> Result<CoinOutput, DispenseError> {
    let resources = wallet
        .get_spendable_resources(AssetId::BASE, amount)
        .await
        .map_err(|e| {
            error(
                format!("Failed to get resources: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    let coin_output = resources
        .into_iter()
        .filter_map(|coin| match coin {
            CoinType::Coin(coin) => Some(CoinOutput {
                utxo_id: coin.utxo_id,
                owner: coin.owner.into(),
                amount: coin.amount,
            }),
            _ => None,
        })
        .last()
        .ok_or_else(|| {
            error(
                "The wallet is empty".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    Ok(coin_output)
}

async fn submit_tx_with_timeout(
    client: &FuelClient,
    tx_id: &Bytes32,
    timeout: u64,
) -> Result<(), DispenseError> {
    tokio::time::timeout(
        Duration::from_secs(timeout),
        client.await_transaction_commit(tx_id),
    )
    .await
    .map(|r| {
        r.map_err(|e| {
            error(
                format!("Failed to submit transaction with error: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
    })
    .map_err(|e| {
        error(
            format!("Got a timeout during transaction submission: {e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })??;

    Ok(())
}

#[tracing::instrument(skip(wallet, config))]
pub async fn dispense_tokens(
    Json(input): Json<DispenseInput>,
    Extension(wallet): Extension<SharedWallet>,
    Extension(state): Extension<SharedFaucetState>,
    Extension(config): Extension<SharedConfig>,
    Extension(client): Extension<Arc<FuelClient>>,
    Extension(network_config): Extension<SharedNetworkConfig>,
    Extension(sessions): Extension<SharedSessions>,
    Extension(dispense_tracker): Extension<SharedDispenseTracker>,
) -> Result<DispenseResponse, DispenseError> {
    let salt: [u8; 32] = hex::decode(&input.salt)
        .and_then(|value| {
            value
                .try_into()
                .map_err(|_| FromHexError::InvalidStringLength)
        })
        .map_err(|_| DispenseError {
            status: StatusCode::BAD_REQUEST,
            error: "Invalid salt".to_string(),
        })?;

    let address = match sessions.lock().await.get(&Salt::new(salt)) {
        Some(value) => *value,
        None => {
            return Err(DispenseError {
                status: StatusCode::NOT_FOUND,
                error: "Salt does not exist".to_string(),
            })
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(input.salt.as_bytes());
    hasher.update(input.nonce.as_bytes());
    let hash: [u8; 32] = hasher.finalize().into();
    let hash_uint = BigUint::from_bytes_be(&hash);

    let u256_max = BigUint::from(2u8).pow(256u32) - BigUint::from(1u8);
    let target_difficulty = u256_max >> config.pow_difficulty;

    if hash_uint > target_difficulty {
        return Err(DispenseError {
            status: StatusCode::NOT_FOUND,
            error: "Invalid proof of work".to_string(),
        });
    }

    check_and_mark_dispense_limit(&dispense_tracker, address, config.dispense_limit_interval)?;
    let cleanup = || {
        dispense_tracker
            .lock()
            .unwrap()
            .remove_in_progress(&address);
    };

    let provider = wallet.provider().expect("client provider");
    let mut tx_id;

    loop {
        let mut guard = state.lock().await;
        let coin_output = if let Some(previous_coin_output) = &guard.last_output {
            *previous_coin_output
        } else {
            get_coin_output(&wallet, config.dispense_amount)
                .await
                .map_err(|e| {
                    cleanup();
                    e
                })?
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

        let inputs = vec![Input::resource_signed(coin_type)];

        let outputs = wallet.get_asset_outputs_for_amount(
            &address.into(),
            config.dispense_asset_id,
            config.dispense_amount,
        );

        let gas_price = guard.next_gas_price();

        let mut script = ScriptTransactionBuilder::prepare_transfer(
            inputs,
            outputs,
            TxPolicies::default().with_gas_price(gas_price),
            network_config.network_info.clone(),
        );

        wallet.sign_transaction(&mut script);

        let script = script.build(provider).await.expect("valid script");

        let total_fee = script
            .fee_checked_from_tx(&network_config.network_info.consensus_parameters)
            .expect("Should be able to calculate fee");

        tx_id = script.id(network_config.network_info.consensus_parameters.chain_id);
        let result = tokio::time::timeout(
            Duration::from_secs(config.timeout),
            provider.send_transaction(script),
        )
        .await
        .map(|r| {
            r.map_err(|e| {
                error(
                    format!("Failed to submit transaction: {e}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })
        })
        .map_err(|e| {
            error(
                format!("Timeout while submitting transaction: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        });

        match result {
            Ok(Ok(_)) => {
                guard.last_output = Some(CoinOutput {
                    utxo_id: UtxoId::new(tx_id, 1),
                    owner: coin_output.owner,
                    amount: coin_output.amount - total_fee.min_fee() - config.dispense_amount,
                });
                break;
            }
            _ => {
                guard.last_output = None;
            }
        };
    }

    submit_tx_with_timeout(&client, &tx_id, config.timeout)
        .await
        .map_err(|e| {
            cleanup();
            e
        })?;

    info!(
        "dispensed {} tokens to {:#x}",
        config.dispense_amount, &address
    );

    dispense_tracker.lock().unwrap().track(address);

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

fn error(error: String, status: StatusCode) -> DispenseError {
    error!("{}", error);
    DispenseError { error, status }
}

impl IntoResponse for CreateSessionResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

impl IntoResponse for CreateSessionError {
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

pub async fn create_session(
    Json(input): Json<CreateSessionInput>,
    Extension(sessions): Extension<SharedSessions>,
    Extension(pow_difficulty): Extension<Arc<u8>>,
    Extension(config): Extension<SharedConfig>,
) -> Result<CreateSessionResponse, CreateSessionError> {
    // parse deposit address
    let address = if let Ok(address) = Address::from_str(input.address.as_str()) {
        Ok(address)
    } else if let Ok(address) = Bech32Address::from_str(input.address.as_str()) {
        Ok(address.into())
    } else {
        return Err(CreateSessionError {
            status: StatusCode::BAD_REQUEST,
            error: "invalid address".to_string(),
        });
    }?;

    // verify captcha
    if let Some(s) = config.captcha_secret.clone() {
        recaptcha::verify(s.expose_secret(), input.captcha.as_str(), None)
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                CreateSessionError {
                    error: "captcha failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            })?;
    }

    let mut map = sessions.lock().await;

    let salt = Salt::random();

    map.insert(salt.clone(), address);

    Ok(CreateSessionResponse {
        status: "Success".to_string(),
        salt: hex::encode(salt.as_bytes()),
        difficulty: *pow_difficulty,
    })
}

#[derive(Deserialize)]
pub struct SessionQuery {
    salt: String,
}

pub async fn get_session(
    query: Query<SessionQuery>,
    Extension(sessions): Extension<SharedSessions>,
) -> Response {
    let salt: Result<[u8; 32], _> = hex::decode(&query.salt).and_then(|value| {
        value
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)
    });

    match salt {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid salt"})),
            )
                .into_response()
        }
    };

    let map = sessions.lock().await;

    let result = map.get(&Salt::new(salt.unwrap()));

    match result {
        Some(address) => (StatusCode::OK, Json(json!({"address": address}))),
        None => (StatusCode::NOT_FOUND, Json(json!({}))),
    }
    .into_response()
}

use crate::{
    models::*, recaptcha, CoinOutput, SharedConfig, SharedDispenseTracker, SharedFaucetState,
    SharedNetworkConfig, SharedWallet,
};
use axum::{
    response::{Html, IntoResponse, Response},
    Extension, Json,
};

use fuel_core_client::client::FuelClient;
use fuel_tx::UtxoId;
use fuel_types::{Address, AssetId};
use fuels_accounts::{Account, Signer, ViewOnlyAccount};
use fuels_core::types::transaction::{Transaction, TxPolicies};
use fuels_core::types::transaction_builders::BuildableTransaction;
use fuels_core::types::{
    bech32::Bech32Address,
    coin::{Coin, CoinStatus},
    coin_type::CoinType,
};
use fuels_core::types::{input::Input, transaction_builders::ScriptTransactionBuilder};
use handlebars::Handlebars;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::{
    collections::BTreeMap,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
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

async fn has_reached_dispense_limit(
    dispense_tracker: &SharedDispenseTracker,
    address: Address,
    interval: u64,
) -> bool {
    let mut dispense_tracker = dispense_tracker.lock().unwrap();
    dispense_tracker.evict_expired_entries(interval);

    if !dispense_tracker.is_tracked(&address) {
        dispense_tracker.mark_in_progress(address);
        return false;
    }

    true
}

#[tracing::instrument(skip(wallet, config))]
pub async fn dispense_tokens(
    Json(input): Json<DispenseInput>,
    Extension(wallet): Extension<SharedWallet>,
    Extension(state): Extension<SharedFaucetState>,
    Extension(config): Extension<SharedConfig>,
    Extension(client): Extension<Arc<FuelClient>>,
    Extension(network_config): Extension<SharedNetworkConfig>,
    Extension(dispense_tracker): Extension<SharedDispenseTracker>,
) -> Result<DispenseResponse, DispenseError> {
    // parse deposit address
    let address = if let Ok(address) = Address::from_str(input.address.as_str()) {
        Ok(address)
    } else if let Ok(address) = Bech32Address::from_str(input.address.as_str()) {
        Ok(address.into())
    } else {
        return Err(error(
            "invalid address".to_string(),
            StatusCode::BAD_REQUEST,
        ));
    }?;

    if has_reached_dispense_limit(&dispense_tracker, address, config.dispense_limit_interval).await
    {
        return Err(error(
            "Account has already received assets today".to_string(),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    };

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

    let mut tx_id;

    loop {
        let mut guard = state.lock().await;
        let coin_output = if let Some(previous_coin_output) = &guard.last_output {
            *previous_coin_output
        } else {
            wallet
                .get_spendable_resources(AssetId::BASE, THE_BIGGEST_AMOUNT)
                .await
                .map_err(|e| {
                    error(
                        format!("Failed to get resources: {e}"),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?
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
                dispense_tracker.lock().unwrap().track(address);

                guard.last_output = Some(CoinOutput {
                    utxo_id: UtxoId::new(tx_id, 1),
                    owner: coin_output.owner,
                    amount: coin_output.amount - total_fee.min_fee() - config.dispense_amount,
                });
                break;
            }
            _ => {
                dispense_tracker
                    .lock()
                    .unwrap()
                    .remove_in_progress(&address);
                guard.last_output = None;
            }
        };
    }

    tokio::time::timeout(
        Duration::from_secs(config.timeout),
        client.await_transaction_commit(&tx_id),
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

fn error(error: String, status: StatusCode) -> DispenseError {
    error!("{}", error);
    DispenseError { error, status }
}

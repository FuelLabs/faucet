use crate::{
    models::*, recaptcha, CoinOutput, SharedConfig, SharedFaucetState, SharedNetworkConfig,
    SharedWallet,
};
use axum::{
    response::{Html, IntoResponse, Response},
    Extension, Json,
};
use fuel_core_client::client::schema::resource::Resource;
use fuel_core_client::client::types::TransactionStatus;
use fuel_tx::{Input, TransactionFee, UniqueIdentifier, UtxoId};
use fuel_types::{Address, AssetId};
use fuels_core::parameters::TxParameters;
use fuels_signers::{Signer, Wallet};
use fuels_types::bech32::Bech32Address;
use handlebars::Handlebars;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde_json::json;
use std::time::Duration;
use std::{
    collections::BTreeMap,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};

// The amount to fetch the biggest input of the faucet.
const THE_BIGGEST_AMOUNT: u64 = (u32::MAX as u64) >> 4;

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

#[memoize::memoize]
pub fn render_page(public_node_url: String) -> String {
    let template = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
    // sub in values
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("index", template)
        .unwrap();
    let mut data = BTreeMap::new();
    data.insert("page_title", "Fuel Faucet");
    data.insert("public_node_url", public_node_url.as_str());
    // render page
    handlebars.render("index", &data).unwrap()
}

pub async fn main(Extension(config): Extension<SharedConfig>) -> Html<String> {
    let public_node_url = config.public_node_url.clone();
    Html(render_page(public_node_url))
}

#[tracing::instrument(skip(wallet))]
pub async fn health(Extension(wallet): Extension<SharedWallet>) -> Response {
    // ping client for health
    let client = wallet
        .get_provider()
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

    let provider = wallet
        .get_provider()
        .map_err(|e| error(format!("Failed to get provider with error: {}", e)))?;

    let tx = {
        let mut guard = state.lock().await;
        let coin_output = if let Some(previous_coin_output) = &guard.last_output {
            *previous_coin_output
        } else {
            provider
                .client
                .resources_to_spend(
                    &wallet.address().hash().to_string(),
                    vec![(
                        AssetId::BASE.to_string().as_str(),
                        THE_BIGGEST_AMOUNT,
                        Some(1),
                    )],
                    None,
                )
                .await
                .map_err(|e| {
                    error!("failed to get resources: {}", e);
                    DispenseError {
                        error: "Failed to get resources".to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                    }
                })?
                .into_iter()
                .flatten()
                .filter_map(|resource| match resource {
                    Resource::Coin(coin) => Some(CoinOutput {
                        utxo_id: coin.utxo_id.into(),
                        owner: coin.owner.into(),
                        amount: coin.amount.into(),
                    }),
                    _ => None,
                })
                .last()
                .expect("The wallet is empty")
        };
        let inputs = vec![Input::coin_signed(
            coin_output.utxo_id,
            coin_output.owner,
            coin_output.amount,
            config.dispense_asset_id,
            Default::default(),
            0,
            Default::default(),
        )];
        let outputs = wallet.get_asset_outputs_for_amount(
            &address.into(),
            config.dispense_asset_id,
            config.dispense_amount,
        );

        let gas_price = guard.next_gas_price();

        let mut tx = Wallet::build_transfer_tx(
            &inputs,
            &outputs,
            TxParameters {
                gas_price,
                ..Default::default()
            },
        );
        wallet.sign_transaction(&mut tx).await.map_err(|e| {
            error!("failed to sign transaction: {}", e);
            DispenseError {
                error: "Failed to sign transaction".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

        let total_fee = TransactionFee::checked_from_tx(&network_config.consensus_parameters, &tx)
            .expect("Can't overflow with transfer transaction")
            .total();

        guard.last_output = Some(CoinOutput {
            utxo_id: UtxoId::new(tx.id(), 1),
            owner: coin_output.owner,
            amount: coin_output.amount - total_fee - config.dispense_amount,
        });

        tx
    };

    let result = tokio::time::timeout(
        Duration::from_secs(config.timeout),
        provider.client.submit_and_await_commit(&tx.into()),
    )
    .await
    .map_err(|e| {
        error(format!(
            "Got a timeout during transaction submission: {}",
            e
        ))
    });

    match result {
        Ok(Ok(TransactionStatus::Success { .. })) => {}
        _ => {
            let mut guard = state.lock().await;
            // If the transaction is not committed or we get an error
            // then we invalidate the state to start from the beginning
            guard.last_output = None;
            result?
                .map_err(|e| error(format!("Failed to submit transaction with error: {}", e)))?;
        }
    };

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

use crate::{
    models::*, CoinOutput, SharedConfig, SharedDispenseTracker, SharedFaucetState,
    SharedNetworkConfig, SharedWallet,
};
use axum::{
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use fuel_core_client::client::FuelClient;
use fuel_tx::UtxoId;
use fuel_types::{AssetId, Bytes32};
use fuels_accounts::{wallet::WalletUnlocked, Account, Signer, ViewOnlyAccount};
use fuels_core::types::{
    bech32::Bech32Address,
    transaction::{Transaction, TxPolicies},
    transaction_builders::BuildableTransaction,
    Address,
};
use fuels_core::types::{
    coin::{Coin, CoinStatus},
    coin_type::CoinType,
};
use fuels_core::types::{input::Input, transaction_builders::ScriptTransactionBuilder};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use std::{str::FromStr, sync::Arc};
use tower_sessions::Session;
use tracing::{error, info};

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
    user_id: String,
    interval: u64,
) -> Result<(), DispenseError> {
    let mut tracker = dispense_tracker.lock().unwrap();
    tracker.evict_expired_entries(interval);

    if tracker.has_tracked(&user_id) {
        return Err(error(
            "Account has already received assets today".to_string(),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }

    tracker.mark_in_progress(user_id);
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

#[derive(Deserialize, Debug)]
pub struct Method {
    pub method: Option<String>,
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all)]
pub async fn tokens_handler(
    Extension(wallet): Extension<SharedWallet>,
    Extension(state): Extension<SharedFaucetState>,
    Extension(config): Extension<SharedConfig>,
    Extension(client): Extension<Arc<FuelClient>>,
    Extension(network_config): Extension<SharedNetworkConfig>,
    Extension(dispense_tracker): Extension<SharedDispenseTracker>,
    session: Session,
    Json(input): Json<DispenseInput>,
) -> Result<DispenseResponse, DispenseError> {
    dispense_auth(
        Extension(wallet),
        Extension(state),
        Extension(config),
        Extension(client),
        Extension(network_config),
        Extension(dispense_tracker),
        session,
        Json(input),
    )
    .await
}

fn get_input<T>(value: Option<T>, prop: &str) -> Result<T, DispenseError> {
    let value =
        value.ok_or_else(|| error(format!("{} is required", prop), StatusCode::BAD_REQUEST))?;
    Ok(value)
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all)]
async fn dispense_auth(
    Extension(wallet): Extension<SharedWallet>,
    Extension(state): Extension<SharedFaucetState>,
    Extension(config): Extension<SharedConfig>,
    Extension(client): Extension<Arc<FuelClient>>,
    Extension(network_config): Extension<SharedNetworkConfig>,
    Extension(dispense_tracker): Extension<SharedDispenseTracker>,
    session: Session,
    Json(input): Json<DispenseInput>,
) -> Result<DispenseResponse, DispenseError> {
    let input_address = get_input(input.address.clone(), "address")?;

    // parse deposit address
    let address = if let Ok(address) = Address::from_str(input_address.as_str()) {
        Ok(address)
    } else if let Ok(address) = Bech32Address::from_str(input_address.as_str()) {
        Ok(address.into())
    } else {
        return Err(error(
            "invalid address".to_string(),
            StatusCode::BAD_REQUEST,
        ));
    }?;

    let user_id: String = session
        .get("user_id")
        .await
        .map_err(|_| {
            error(
                "Loading the session failed".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?
        .ok_or_else(|| {
            error(
                "Session is missing user_id".to_string(),
                StatusCode::UNAUTHORIZED,
            )
        })?;

    check_and_mark_dispense_limit(
        &dispense_tracker,
        user_id.clone(),
        config.dispense_limit_interval,
    )?;
    let cleanup = || {
        dispense_tracker
            .lock()
            .unwrap()
            .remove_in_progress(&user_id);
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

    dispense_tracker.lock().unwrap().track(user_id);

    Ok(DispenseResponse {
        status: "Success".to_string(),
        tokens: config.dispense_amount,
        tx_id: tx_id.to_string(),
    })
}

#[tracing::instrument(skip_all)]
pub async fn info_handler(
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

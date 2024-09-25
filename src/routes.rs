use crate::{
    models::*, recaptcha, CoinOutput, SharedConfig, SharedDispenseTracker, SharedFaucetState,
    SharedWallet,
};
use axum::{
    response::{Html, IntoResponse, Response},
    Extension, Json,
};
use fuel_core_client::client::{types::NodeInfo, FuelClient};
use fuel_tx::{Output, UtxoId};
use fuel_types::{Address, AssetId, Bytes32};
use fuels_accounts::{wallet::WalletUnlocked, Account, ViewOnlyAccount};
use fuels_core::types::{
    bech32::Bech32Address,
    coin::{Coin, CoinStatus},
    coin_type::CoinType,
    input::Input,
    transaction::{Transaction, TxPolicies},
    transaction_builders::{BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder},
};
use handlebars::Handlebars;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde_json::json;
use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

#[memoize::memoize]
pub fn render_page(public_node_url: String, captcha_key: Option<String>) -> String {
    let template = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("index", template)
        .unwrap();
    let mut data = BTreeMap::new();
    data.insert("page_title", "Fuel Faucet");
    data.insert("public_node_url", public_node_url.as_str());
    if let Some(captcha_key) = &captcha_key {
        data.insert("captcha_key", captcha_key.as_str());
    }
    handlebars.render("index", &data).unwrap()
}

pub async fn main(Extension(config): Extension<SharedConfig>) -> Html<String> {
    Html(render_page(config.public_node_url.clone(), config.captcha_key.clone()))
}

#[tracing::instrument(skip_all)]
pub async fn health(Extension(wallet): Extension<SharedWallet>) -> Response {
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
        (self.status, Json(json!({ "error": self.error }))).into_response()
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

    if tracker.is_in_progress(&address) {
        return Err(error(
            "Account is already in the process of receiving assets".to_string(),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }

    tracker.mark_in_progress(address);
    Ok(())
}

async fn get_coins(
    wallet: &WalletUnlocked,
    base_asset_id: &AssetId,
    amount: u64,
) -> Result<Vec<Input>, DispenseError> {
    wallet
        .get_spendable_resources(*base_asset_id, amount)
        .await
        .map_err(|e| {
            error(
                format!("Failed to get resources: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
        .map(|resources| resources.into_iter().map(Input::resource_signed).collect())
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
    .map_err(|e| {
        error(
            format!("Got a timeout during transaction submission: {e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?
    .map_err(|e| {
        error(
            format!("Failed to submit transaction with error: {e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })
}

#[tracing::instrument(skip_all)]
pub async fn dispense_tokens(
    Json(input): Json<DispenseInput>,
    Extension(wallet): Extension<SharedWallet>,
    Extension(state): Extension<SharedFaucetState>,
    Extension(config): Extension<SharedConfig>,
    Extension(info_node): Extension<Arc<NodeInfo>>,
    Extension(client): Extension<Arc<FuelClient>>,
    Extension(dispense_tracker): Extension<SharedDispenseTracker>,
) -> Result<DispenseResponse, DispenseError> {
    let address = parse_address(&input.address)?;

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

    check_and_mark_dispense_limit(&dispense_tracker, address, config.dispense_limit_interval)?;

    let _cleanup = CleanUpper(|| {
        dispense_tracker
            .lock()
            .unwrap()
            .remove_in_progress(&address);
    });

    let provider = wallet.provider().expect("client provider");
    let base_asset_id = *provider.consensus_parameters().base_asset_id();

    let tx_id = retry_transaction(
        &wallet,
        &state,
        &config,
        &info_node,
        &provider,
        &base_asset_id,
        address,
    )
    .await?;

    submit_tx_with_timeout(&client, &tx_id, config.timeout).await?;

    info!(
        "dispensed {} tokens to {:#x}",
        config.dispense_amount, &address
    );

    let mut tracker = dispense_tracker.lock().unwrap();
    tracker.track(address);

    Ok(DispenseResponse {
        status: "Success".to_string(),
        tokens: config.dispense_amount,
        tx_id: tx_id.to_string(),
    })
}

#[tracing::instrument(skip_all)]
pub async fn dispense_info(
    Extension(config): Extension<SharedConfig>,
    Extension(wallet): Extension<SharedWallet>,
) -> Result<DispenseInfoResponse, DispenseError> {
    let provider = wallet.provider().expect("client provider");
    let base_asset_id = *provider.consensus_parameters().base_asset_id();

    Ok(DispenseInfoResponse {
        amount: config.dispense_amount,
        asset_id: base_asset_id.to_string(),
    })
}

fn error(error: String, status: StatusCode) -> DispenseError {
    error!("{}", error);
    DispenseError { error, status }
}

fn available_balance(inputs: &[Input], base_asset_id: &AssetId) -> u64 {
    inputs
        .iter()
        .filter_map(|input| match input {
            Input::ResourceSigned { resource, .. } | Input::ResourcePredicate { resource, .. } => {
                match resource {
                    CoinType::Coin(Coin {
                        amount, asset_id, ..
                    }) if asset_id == base_asset_id => Some(*amount),
                    CoinType::Message(message) => Some(message.amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .sum()
}

struct CleanUpper<F: FnMut()>(F);

impl<F: FnMut()> Drop for CleanUpper<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

fn parse_address(address_str: &str) -> Result<Address, DispenseError> {
    Address::from_str(address_str)
        .or_else(|_| Bech32Address::from_str(address_str).map(Into::into))
        .map_err(|_| error("invalid address".to_string(), StatusCode::BAD_REQUEST))
}

async fn retry_transaction(
    wallet: &WalletUnlocked,
    state: &SharedFaucetState,
    config: &SharedConfig,
    info_node: &NodeInfo,
    provider: &impl fuels_core::provider::Provider,
    base_asset_id: &AssetId,
    address: Address,
) -> Result<Bytes32, DispenseError> {
    for _ in 0..config.number_of_retries {
        let mut guard = state.lock().await;
        let inputs = get_transaction_inputs(wallet, &guard, config, info_node, base_asset_id).await?;
        let outputs = get_transaction_outputs(address, wallet.address().into(), config.dispense_amount, *base_asset_id);
        
        let tip = guard.next_tip();
        let mut tx_builder = ScriptTransactionBuilder::prepare_transfer(
            inputs,
            outputs,
            TxPolicies::default().with_tip(tip),
        );

        wallet.add_witnesses(&mut tx_builder).expect("Valid witness");
        wallet
            .adjust_for_fee(&mut tx_builder, config.dispense_amount)
            .await
            .map_err(|e| error(format!("Failed to adjust for fee: {e}"), StatusCode::INTERNAL_SERVER_ERROR))?;

        let (fee, stable_fee_change) = calculate_fee_and_change(&tx_builder, provider, config.dispense_amount, base_asset_id).await?;

        *tx_builder.outputs.last_mut().unwrap() = Output::coin(wallet.address().into(), stable_fee_change, *base_asset_id);

        let script = tx_builder.build(provider).await.expect("Valid script");
        let id = script.id(provider.chain_id());

        match submit_transaction(provider, script, config.timeout, address).await {
            Ok(_) => {
                guard.last_output = Some(CoinOutput {
                    utxo_id: UtxoId::new(id, 2),
                    owner: wallet.address().into(),
                    amount: stable_fee_change,
                });
                return Ok(id);
            }
            Err(e) => {
                tracing::warn!("{}", e);
                guard.last_output = None;
            }
        };
    }

    Err(error(
        "Failed to submit transaction".to_string(),
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

async fn get_transaction_inputs(
    wallet: &WalletUnlocked,
    guard: &mut tokio::sync::MutexGuard<'_, FaucetState>,
    config: &SharedConfig,
    info_node: &NodeInfo,
    base_asset_id: &AssetId,
) -> Result<Vec<Input>, DispenseError> {
    let amount = guard.last_output.as_ref().map_or(0, |o| o.amount);
    if amount > config.dispense_amount {
        let previous_coin_output = guard.last_output.as_ref().expect("Checked above");
        Ok(vec![Input::resource_signed(CoinType::Coin(Coin {
            amount: previous_coin_output.amount,
            block_created: 0u32,
            asset_id: *base_asset_id,
            utxo_id: previous_coin_output.utxo_id,
            owner: previous_coin_output.owner.into(),
            status: CoinStatus::Unspent,
        }))])
    } else {
        get_coins(
            wallet,
            base_asset_id,
            config.dispense_amount * info_node.max_depth * 2,
        )
        .await
    }
}

fn get_transaction_outputs(recipient_address: Address, faucet_address: Address, dispense_amount: u64, base_asset_id: AssetId) -> Vec<Output> {
    vec![
        Output::coin(recipient_address, dispense_amount, base_asset_id),
        Output::change(recipient_address, 0, base_asset_id),
        Output::coin(faucet_address, 0, base_asset_id),
    ]
}

async fn calculate_fee_and_change(
    tx_builder: &ScriptTransactionBuilder,
    provider: &impl fuels_core::provider::Provider,
    dispense_amount: u64,
    base_asset_id: &AssetId,
) -> Result<(u64, u64), DispenseError> {
    let fee = tx_builder
        .fee_checked_from_tx(provider)
        .await
        .map_err(|e| error(format!("Error calculating `TransactionFee`: {e}"), StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| error("Overflow during calculating `TransactionFee`".to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    let available_balance = available_balance(&tx_builder.inputs, base_asset_id);
    let stable_fee_change = available_balance
        .checked_sub(fee.max_fee().saturating_add(dispense_amount))
        .ok_or_else(|| error("Not enough asset to cover a max fee".to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok((fee.max_fee(), stable_fee_change))
}

async fn submit_transaction(
    provider: &impl fuels_core::provider::Provider,
    script: Transaction,
    timeout: u64,
    address: Address,
) -> Result<(), DispenseError> {
    tokio::time::timeout(Duration::from_secs(timeout), provider.send_transaction(script))
        .await
        .map_err(|_| error(format!("Timeout while submitting transaction for address: {address:X}"), StatusCode::INTERNAL_SERVER_ERROR))?
            .map_err(|e| error(
            format!(
                "Failed to submit transaction for address: {address:X} with error: {}",
                e
            ),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
}

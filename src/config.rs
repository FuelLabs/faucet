use crate::constants::{
    CAPTCHA_SECRET, DEFAULT_FAUCET_DISPENSE_AMOUNT, DEFAULT_MAX_DISPENSES_PER_MINUTE,
    DEFAULT_NODE_URL, DEFAULT_PORT, FAUCET_ASSET_ID, FAUCET_DISPENSE_AMOUNT, FUEL_NODE_URL,
    HUMAN_LOGGING, LOG_FILTER, MAX_DISPENSES_PER_MINUTE, MIN_BYTE_PRICE, MIN_GAS_PRICE,
    SERVICE_PORT, WALLET_SECRET_KEY,
};
use fuel_types::AssetId;
use secrecy::Secret;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub log_filter: String,
    pub human_logging: bool,
    pub service_port: u16,
    pub captcha_secret: Option<Secret<String>>,
    pub node_url: String,
    pub wallet_secret_key: Option<Secret<String>>,
    pub fuel_dispense_amount: u64,
    pub dispense_asset_id: AssetId,
    pub max_dispenses_per_minute: u64,
    pub min_gas_price: u64,
    pub min_byte_price: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_filter: env::var(LOG_FILTER).unwrap_or_default(),
            human_logging: parse_bool(HUMAN_LOGGING, true),
            service_port: env::var_os(SERVICE_PORT)
                .map(|s| s.into_string().unwrap().parse().unwrap())
                .unwrap_or(DEFAULT_PORT),
            captcha_secret: env::var_os(CAPTCHA_SECRET)
                .map(|s| Secret::new(s.into_string().unwrap())),
            node_url: env::var(FUEL_NODE_URL).unwrap_or_else(|_| DEFAULT_NODE_URL.to_string()),
            wallet_secret_key: env::var_os(WALLET_SECRET_KEY)
                .map(|s| Secret::new(s.into_string().unwrap())),
            fuel_dispense_amount: env::var(FAUCET_DISPENSE_AMOUNT)
                .unwrap_or_else(|_| DEFAULT_FAUCET_DISPENSE_AMOUNT.to_string())
                .parse::<u64>()
                .expect("expected a valid integer for FAUCET_DISPENSE_AMOUNT"),
            dispense_asset_id: FAUCET_ASSET_ID,
            max_dispenses_per_minute: env::var(MAX_DISPENSES_PER_MINUTE)
                .unwrap_or_else(|_| DEFAULT_MAX_DISPENSES_PER_MINUTE.to_string())
                .parse::<u64>()
                .expect("expected a valid integer for MAX_DISPENSES_PER_MINUTE"),
            min_gas_price: env::var(MIN_GAS_PRICE)
                .unwrap_or_else(|_| "0".to_string())
                .parse::<u64>()
                .expect("expected a valid integer for MIN_GAS_PRICE"),
            min_byte_price: env::var(MIN_BYTE_PRICE)
                .unwrap_or_else(|_| "0".to_string())
                .parse::<u64>()
                .expect("expected a valid integer for MIN_BYTE_PRICE"),
        }
    }
}

fn parse_bool(env_var: &str, default: bool) -> bool {
    env::var_os(env_var)
        .map(|s| {
            s.to_str().unwrap().parse().unwrap_or_else(|_| {
                panic!(
                    "Expected `true` or `false` to be provided for `{}`",
                    env_var
                )
            })
        })
        .unwrap_or(default)
}

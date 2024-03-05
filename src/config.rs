use crate::constants::{
    CAPTCHA_KEY, CAPTCHA_SECRET, CLERK_KEY, DEFAULT_DISPENSE_INTERVAL,
    DEFAULT_FAUCET_DISPENSE_AMOUNT, DEFAULT_NODE_URL, DEFAULT_PORT, DISPENSE_AMOUNT,
    DISPENSE_INTERVAL, FAUCET_ASSET_ID, FUEL_NODE_URL, HUMAN_LOGGING, LOG_FILTER, MIN_GAS_PRICE,
    PUBLIC_FUEL_NODE_URL, SERVICE_PORT, TIMEOUT_SECONDS, WALLET_SECRET_KEY,
};
use fuels_core::types::AssetId;
use secrecy::Secret;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub log_filter: String,
    pub human_logging: bool,
    pub service_port: u16,
    pub captcha_key: Option<String>,
    pub captcha_secret: Option<Secret<String>>,
    pub clerk_key: Option<String>,
    pub node_url: String,
    pub public_node_url: String,
    pub wallet_secret_key: Option<Secret<String>>,
    pub dispense_amount: u64,
    pub dispense_asset_id: AssetId,
    pub dispense_limit_interval: u64,
    pub min_gas_price: u64,
    pub timeout: u64,
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
            captcha_key: env::var_os(CAPTCHA_KEY).map(|s| s.into_string().unwrap()),
            clerk_key: env::var_os(CLERK_KEY).map(|s| s.into_string().unwrap()),
            node_url: env::var(FUEL_NODE_URL).unwrap_or_else(|_| DEFAULT_NODE_URL.to_string()),
            public_node_url: env::var(PUBLIC_FUEL_NODE_URL)
                .unwrap_or_else(|_| DEFAULT_NODE_URL.to_string()),
            wallet_secret_key: env::var_os(WALLET_SECRET_KEY)
                .map(|s| Secret::new(s.into_string().unwrap())),
            dispense_amount: env::var(DISPENSE_AMOUNT)
                .unwrap_or_else(|_| DEFAULT_FAUCET_DISPENSE_AMOUNT.to_string())
                .parse::<u64>()
                .expect("expected a valid integer for DISPENSE_AMOUNT"),
            dispense_asset_id: FAUCET_ASSET_ID,
            dispense_limit_interval: env::var(DISPENSE_INTERVAL)
                .unwrap_or_else(|_| DEFAULT_DISPENSE_INTERVAL.to_string())
                .parse::<u64>()
                .expect("expected a valid integer for DISPENSE_LIMIT_INTERVAL"),
            min_gas_price: env::var(MIN_GAS_PRICE)
                .unwrap_or_else(|_| "0".to_string())
                .parse::<u64>()
                .expect("expected a valid integer for MIN_GAS_PRICE"),
            timeout: env::var(TIMEOUT_SECONDS)
                .unwrap_or_else(|_| "30".to_string())
                .parse::<u64>()
                .expect("expected a valid integer for TIMEOUT_SECONDS"),
        }
    }
}

fn parse_bool(env_var: &str, default: bool) -> bool {
    env::var_os(env_var)
        .map(|s| {
            s.to_str().unwrap().parse().unwrap_or_else(|_| {
                panic!("Expected `true` or `false` to be provided for `{env_var}`")
            })
        })
        .unwrap_or(default)
}

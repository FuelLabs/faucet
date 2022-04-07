use crate::constants::{
    CAPTCHA_SECRET, DEFAULT_NODE_URL, DEFAULT_PORT, FAUCET_ASSET_ID, FAUCET_DISPENSE_AMOUNT,
    FUEL_NODE_URL, HUMAN_LOGGING, LOG_FILTER, SERVICE_PORT, WALLET_SECRET_KEY,
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
            node_url: env::var(FUEL_NODE_URL).unwrap_or(DEFAULT_NODE_URL.to_string()),
            wallet_secret_key: env::var_os(WALLET_SECRET_KEY)
                .map(|s| Secret::new(s.into_string().unwrap())),
            fuel_dispense_amount: FAUCET_DISPENSE_AMOUNT,
            dispense_asset_id: FAUCET_ASSET_ID,
        }
    }
}

fn parse_bool(env_var: &str, default: bool) -> bool {
    env::var_os(env_var)
        .map(|s| {
            s.to_str().unwrap().parse().expect(
                format!(
                    "Expected `true` or `false` to be provided for `{}`",
                    env_var
                )
                .as_str(),
            )
        })
        .unwrap_or(default)
}

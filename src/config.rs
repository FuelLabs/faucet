use crate::constants::{
    CAPTCHA_KEY, CAPTCHA_SECRET, DEFAULT_DISPENSE_INTERVAL, DEFAULT_FAUCET_DISPENSE_AMOUNT,
    DEFAULT_NODE_URL, DEFAULT_NUMBER_OF_RETRIES, DEFAULT_PORT, DISPENSE_AMOUNT, DISPENSE_INTERVAL,
    FUEL_NODE_URL, HUMAN_LOGGING, LOG_FILTER, NUMBER_OF_RETRIES, PUBLIC_FUEL_NODE_URL,
    SERVICE_PORT, TIMEOUT_SECONDS, WALLET_SECRET_KEY,
};
use secrecy::Secret;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub log_filter: String,
    pub human_logging: bool,
    pub service_port: u16,
    pub captcha_key: Option<String>,
    pub captcha_secret: Option<Secret<String>>,
    pub node_url: String,
    pub public_node_url: String,
    pub wallet_secret_key: Option<Secret<String>>,
    pub dispense_amount: u64,
    pub number_of_retries: u64,
    pub dispense_limit_interval: u64,
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
            node_url: env::var(FUEL_NODE_URL).unwrap_or_else(|_| DEFAULT_NODE_URL.to_string()),
            public_node_url: env::var(PUBLIC_FUEL_NODE_URL)
                .unwrap_or_else(|_| DEFAULT_NODE_URL.to_string()),
            wallet_secret_key: env::var_os(WALLET_SECRET_KEY)
                .map(|s| Secret::new(s.into_string().unwrap())),
            dispense_amount: env::var(DISPENSE_AMOUNT)
                .unwrap_or_else(|_| DEFAULT_FAUCET_DISPENSE_AMOUNT.to_string())
                .parse::<u64>()
                .expect("expected a valid integer for DISPENSE_AMOUNT"),
            number_of_retries: env::var(NUMBER_OF_RETRIES)
                .unwrap_or_else(|_| DEFAULT_NUMBER_OF_RETRIES.to_string())
                .parse::<u64>()
                .expect("expected a valid integer for NUMBER_OF_RETRIES"),
            dispense_limit_interval: env::var(DISPENSE_INTERVAL)
                .unwrap_or_else(|_| DEFAULT_DISPENSE_INTERVAL.to_string())
                .parse::<u64>()
                .expect("expected a valid integer for DISPENSE_LIMIT_INTERVAL"),
            timeout: env::var(TIMEOUT_SECONDS)
                .unwrap_or_else(|_| "10".to_string())
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

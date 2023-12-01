use fuels_core::types::AssetId;

pub const LOG_FILTER: &str = "RUST_LOG";
pub const HUMAN_LOGGING: &str = "HUMAN_LOGGING";
pub const CAPTCHA_KEY: &str = "CAPTCHA_KEY";
pub const CAPTCHA_SECRET: &str = "CAPTCHA_SECRET";
pub const WALLET_SECRET_KEY: &str = "WALLET_SECRET_KEY";
pub const PUBLIC_FUEL_NODE_URL: &str = "PUBLIC_FUEL_NODE_URL";
pub const WALLET_SECRET_DEV_KEY: &str =
    "99ad179d4f892ff3124ccd817408ff8a4452d9c16bb1b4968b8a59797e13cd7a";
pub const FUEL_NODE_URL: &str = "FUEL_NODE_URL";
pub const DEFAULT_NODE_URL: &str = "http://127.0.0.1:4000";
pub const DISPENSE_AMOUNT: &str = "DISPENSE_AMOUNT";
pub const DISPENSE_INTERVAL: &str = "DISPENSE_LIMIT_INTERVAL";
pub const DEFAULT_DISPENSE_INTERVAL: u64 = 24 * 60 * 60;
pub const DEFAULT_FAUCET_DISPENSE_AMOUNT: u64 = 10_000_000;
pub const FAUCET_ASSET_ID: AssetId = AssetId::new([0; 32]);
pub const SERVICE_PORT: &str = "PORT";
pub const DEFAULT_PORT: u16 = 3000;

pub const MIN_GAS_PRICE: &str = "MIN_GAS_PRICE";
pub const TIMEOUT_SECONDS: &str = "TIMEOUT_SECONDS";
pub const POW_DIFFICULTY: &str = "POW_DIFFICULTY";

// HTTP config

/// The max number of simultaneous requests that can be buffered until backpressure is applied
pub const MAX_CONCURRENT_REQUESTS: usize = 1024usize;

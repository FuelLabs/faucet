use fuels_core::types::AssetId;

pub const LOG_FILTER: &str = "RUST_LOG";
pub const HUMAN_LOGGING: &str = "HUMAN_LOGGING";
pub const CAPTCHA_KEY: &str = "CAPTCHA_KEY";
pub const CAPTCHA_SECRET: &str = "CAPTCHA_SECRET";
pub const WALLET_SECRET_KEY: &str = "WALLET_SECRET_KEY";
pub const CLERK_PUB_KEY: &str = "CLERK_PUB_KEY";
pub const CLERK_SECRET_KEY: &str = "CLERK_SECRET_KEY";
pub const PUBLIC_FUEL_NODE_URL: &str = "PUBLIC_FUEL_NODE_URL";
pub const WALLET_SECRET_DEV_KEY: &str =
    "a449b1ffee0e2205fa924c6740cc48b3b473aa28587df6dab12abc245d1f5298";
pub const FUEL_NODE_URL: &str = "FUEL_NODE_URL";
pub const DEFAULT_NODE_URL: &str = "http://127.0.0.1:4000";
pub const DISPENSE_AMOUNT: &str = "DISPENSE_AMOUNT";
pub const DISPENSE_INTERVAL: &str = "DISPENSE_LIMIT_INTERVAL";
pub const DEFAULT_DISPENSE_INTERVAL: u64 = 24 * 60 * 60;
pub const DEFAULT_FAUCET_DISPENSE_AMOUNT: u64 = 10_000_000;
pub const FAUCET_ASSET_ID: AssetId = AssetId::new([0; 32]);
pub const SERVICE_PORT: &str = "PORT";
pub const DEFAULT_PORT: u16 = 3001;

pub const MIN_GAS_PRICE: &str = "MIN_GAS_PRICE";
pub const TIMEOUT_SECONDS: &str = "TIMEOUT_SECONDS";

// HTTP config

/// The max number of simultaneous requests that can be buffered until backpressure is applied
pub const MAX_CONCURRENT_REQUESTS: usize = 1024usize;

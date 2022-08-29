use tracing_subscriber::EnvFilter;
use fuel_faucet::{config::Config, start_server};

#[tokio::main]
async fn main() {
    let config = Config::default();
    init_logger(&config);
    let (_, task) = start_server(config).await;
    let _ = task.await.unwrap();
}

fn init_logger(config: &Config) {
    let filter = if !config.log_filter.is_empty() {
        EnvFilter::try_from_default_env().expect("Invalid `RUST_LOG` provided")
    } else {
        EnvFilter::new("info")
    };

    let sub = tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(std::io::stderr)
        .with_env_filter(filter);

    if config.human_logging {
        // use pretty logs
        sub.init();
    } else {
        // use machine parseable structured logs
        sub
            // disable terminal colors
            .with_ansi(false)
            // use json
            .json()
            .init();
    }
}
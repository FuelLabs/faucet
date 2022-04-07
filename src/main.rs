use fuel_faucet::{config::Config, start_server};

#[tokio::main]
async fn main() {
    let (_, task) = start_server(Config::default()).await;
    let _ = task.await.unwrap();
}

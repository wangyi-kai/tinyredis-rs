use redis_rs::client::client::run_client;

#[tokio::main]
async fn main() {
    run_client().await;
}
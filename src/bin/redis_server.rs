use redis_rs::server::server::run_server;

use tokio::{net::TcpListener, signal};
use redis_rs::{DB_SIZE, DEFAULT_PORT};

#[tokio::main]
async fn main() {
    let port = DEFAULT_PORT;
    let listener = TcpListener::bind(&format!("0.0.0.0:{}", port)).await.unwrap();
    run_server(listener, signal::ctrl_c(), DB_SIZE as u32).await;
}
use std::fmt::Write;

use redis_rs::db::data_structure::dict::lib::random_i32;
use redis_rs::client::client::Client;
use redis_rs::Result;
use hdrhistogram::Histogram;
use tokio::time::Instant;
use redis_rs::error::Error;

#[derive(Debug)]
pub struct BenchmarkConfig {
    host: String,
    port: u16,
    num_clients: u32,
    requests: u32,
    key_size: u32,
    data_size: u32,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
            num_clients: 50,
            requests: 100000,
            key_size: 20,
            data_size: 3,
        }
    }
}

fn gen_benchmark_data(count: usize) -> String {
    let mut state: u32 = 1234;
    let mut data = String::with_capacity(count);

    for _ in 0..count {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let ch = b'0' + ((state >> 16) & 63) as u8;
        data.push(ch as char);
    }
    data
}

pub fn num_to_str(value: i64) -> String {
        let mut s = String::with_capacity(32);
        let _ = write!(&mut s, "{}", value);
        s
    }

pub fn gen_command(cmd_type: &str, data: &str) -> String {
    match cmd_type {
        "set" => {
            let key = num_to_str(random_i32() as i64);
            format!("set key{} {}", key, data.clone())
        }
        "get" => {
            let key = num_to_str(random_i32() as i64);
            format!("get key{}", key)
        }
        "hset" => {
            let key = num_to_str(random_i32() as i64);
            format!("hset myhash element{} {}", key, data.clone())
        }
        "hget" => {
            let key = num_to_str(random_i32() as i64);
            format!("hget myhash element{}", key)
        }
        _ => String::default()
    }
}

pub async fn create_client(config: &BenchmarkConfig) -> Client {
    let host = config.host.to_string();
    let port = config.port;
    let addr = format!("{}:{}", host, port);
    Client::connect(addr).await.unwrap()
}

pub async fn benchmark(client: &mut Client, cmd: String) -> Result<()> {
    let start = Instant::now();
    client.benchmark_send_command(&cmd).await.unwrap();
    let res = client.benchmark_receive().await?;
    match res {
        Some(_) => {
            let end = start.elapsed().as_millis();
            println!("Latency: {}ms", end);
            Ok(()) }
        None => Err(Error::ReceiveErr(-1).into())
    }
}

#[tokio::main]
async fn main() {
    let config = BenchmarkConfig::default();
    let mut client = create_client(&config).await;
    let mut hist = Histogram::<u64>::new_with_max(10_000_000, 3).unwrap();

    let data = gen_benchmark_data(100);
    for _i in 0..1000 {
        let cmd = gen_command("set", &data);
        benchmark(&mut client, cmd).await;
    }
}
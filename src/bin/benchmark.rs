use std::fmt::Write;
use std::sync::{Arc};
use redis_rs::db::data_structure::dict::lib::random_i32;
use redis_rs::client::client::{Client, Tokens};
use redis_rs::Result;

use hdrhistogram::Histogram;
use tokio::time::Instant;
use tokio::sync::mpsc;
use clap::Parser;
use redis_rs::parser::cmd::command::CommandStrategy;
use redis_rs::server::connection::Connection;

#[derive(Parser, Clone)]
struct BenchmarkConfig {
    #[arg(short, long, default_value = "127.0.0.1")]
    pub ip: String,
    #[arg(short, long, default_value_t = 8000)]
    pub port: u16,
    #[arg(short, long, default_value_t = 50)]
    pub clients: u32,
    #[arg(short, long, default_value_t = 100000)]
    pub requests: u32,
    #[arg(short, long, default_value_t = 3)]
    pub data_size: u32,
    #[arg(short, long, num_args = 1..)]
    pub tests: Vec<String>,
}

unsafe impl Sync for BenchmarkConfig {}
unsafe impl Send for BenchmarkConfig {}

fn gen_benchmark_data(count: u32) -> String {
    let mut state: u32 = 1234;
    let mut data = String::with_capacity(count as usize);

    for _ in 0..count {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let ch = b'0' + ((state >> 16) & 63) as u8;
        data.push(ch as char);
    }
    data
}

fn test_is_selected(tests: &Vec<String>, name: &String) -> bool {
    if tests.is_empty() {
        true
    } else {
        tests.contains(name)
    }
}

fn num_to_str(value: i64) -> String {
        let mut s = String::with_capacity(32);
        let _ = write!(&mut s, "{}", value);
        s
    }

fn gen_command(cmd_type: &str, data: &str) -> String {
    match cmd_type {
        "ping" => {
            "ping".to_string()
        }
        "set" => {
            let key = num_to_str(random_i32() as i64);
            format!("set key{} {}", key, data)
        }
        "get" => {
            let key = num_to_str(random_i32() as i64);
            format!("get key{}", key)
        }
        "hset" => {
            let key = num_to_str(random_i32() as i64);
            format!("hset myhash element{} {}", key, data)
        }
        "hget" => {
            let key = num_to_str(random_i32() as i64);
            format!("hget myhash element{}", key)
        }
        _ => String::default()
    }
}

fn cmd_to_bytes(cmd: &str, data: &str) -> Vec<u8> {
    let cmd = gen_command(cmd, data);
    let tokens = Tokens::from(&cmd);
    let redis_cmd = tokens.to_command().unwrap();
    let frame = redis_cmd.into_frame();
    let mut buf = vec![];
    Connection::write_value(&frame, &mut buf);
    buf
}

async fn create_client(config: Arc<BenchmarkConfig>) -> Client {
    let host = config.ip.clone();
    let port = config.port;
    let addr = format!("{}:{}", host, port);
    Client::connect(addr).await.unwrap()
}

pub async fn benchmark_pipe(cmd: &str, config: Arc<BenchmarkConfig>) -> Result<()> {
    println!("======{}======", cmd.to_uppercase());
    let mut hist = Histogram::<u64>::new_with_bounds(1, 3_600_000, 3).unwrap();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(config.clients as usize);
    let data = gen_benchmark_data(config.data_size);
    for _ in 0..config.clients {
        let mut cmd_vec = Vec::with_capacity(config.clients as usize);
        for _ in 0..config.requests {
            let buf = cmd_to_bytes(cmd, &data);
            cmd_vec.extend_from_slice(&buf);
        }
        let _ = tx.send(cmd_vec).await;
    }

    let mut handles = Vec::with_capacity(config.clients as usize);

    let st = Instant::now();
    for _i in 0..config.clients {
        let config = config.clone();
        let requests = config.requests;
        let cmd = rx.recv().await.unwrap();
        let mut client = create_client(config).await;
        let handle = tokio::spawn(async move {
            let _ = client.benchmark_send_command(cmd).await;
            for _i in 0..requests {
                let _res = client.benchmark_receive().await;
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.await;
    }
    let end = st.elapsed().as_millis() as f64;
    let req_per_sec = config.requests as f64 / end ;
    println!(" {:.3} requests completed in {} seconds", config.requests, end / 1000f64);
    println!(" {} parallel clients", config.clients);
    println!(" Latency by percentile distribution:");
    hist.record(end as u64).unwrap();
    println!(" Summary:");
    println!("     Throughput summary: {:.2} requests per second", req_per_sec * 1000f64);
    println!("     Latency summary (sec): ");
    println!("               {}  {}   {}   {}  {}  {}", "avg", "min", "p50", "p95", "p99", "max");
    println!("               {}  {}   {} {} {} {}", 0, hist.min(), hist.value_at_quantile(0.5) as f64, hist.value_at_quantile(0.95), hist.value_at_quantile(0.99), hist.max());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: BenchmarkConfig = BenchmarkConfig::parse();
    let cmd = &config.tests;
    let config = Arc::new(config.clone());
    if test_is_selected(cmd, &"ping".to_string()) {
        benchmark_pipe("ping", config.clone()).await?;
    }
    if test_is_selected(cmd, &"set".to_string()) {
        benchmark_pipe("set", config.clone()).await?;
    }
    if test_is_selected(cmd, &"get".to_string()) {
        benchmark_pipe("get", config.clone()).await?;
    }
    if test_is_selected(cmd, &"hset".to_string()) {
        benchmark_pipe("hset", config.clone()).await?;
    }

    Ok(())
}
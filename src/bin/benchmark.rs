use std::fmt::Write;
use std::sync::{Arc};
use redis_rs::db::data_structure::dict::lib::random_i32;
use redis_rs::client::client::{Client, Tokens};
use redis_rs::Result;

use hdrhistogram::Histogram;
use tokio::time::Instant;
use tokio::sync::mpsc;
use redis_rs::parser::cmd::command::CommandStrategy;
use redis_rs::server::connection::Connection;

#[derive(Debug, Clone)]
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
            data_size: 2,
        }
    }
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

pub fn num_to_str(value: i64) -> String {
        let mut s = String::with_capacity(32);
        let _ = write!(&mut s, "{}", value);
        s
    }

pub fn gen_command(cmd_type: &str, data: &str) -> String {
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

pub async fn create_client(config: Arc<BenchmarkConfig>) -> Client {
    let host = config.host.clone();
    let port = config.port;
    let addr = format!("{}:{}", host, port);
    Client::connect(addr).await.unwrap()
}


pub async fn benchmark_pipe(cmd: &str, config: Arc<BenchmarkConfig>) -> Result<()> {
    println!("======{}======", cmd.to_uppercase());
    let mut hist = Histogram::<u64>::new_with_bounds(1, 3_600_000, 3).unwrap();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(config.num_clients as usize);
    let data = gen_benchmark_data(config.data_size);
    for _ in 0..config.num_clients {
        let mut cmd_vec = Vec::with_capacity(config.num_clients as usize);
        for _ in 0..config.requests {
            let cmd = gen_command(cmd, &data);
            let tokens = Tokens::from(&cmd);
            let redis_cmd = tokens.to_command()?;
            let frame = redis_cmd.into_frame();
            let mut buf = vec![];
            Connection::write_value(&frame, &mut buf);
            cmd_vec.extend_from_slice(&buf);
        }
        let _ = tx.send(cmd_vec).await;
    }

    let mut handles = Vec::with_capacity(config.num_clients as usize);

    let st = Instant::now();
    for _i in 0..config.num_clients {
        let config = config.clone();
        let requests = config.requests;
        let cmd = rx.recv().await.unwrap();
        let handle = tokio::spawn(async move {
            let mut client = create_client(config).await;
            let _ = client.benchmark_send_command(cmd).await;
            for _i in 0..requests{
                let _res = client.benchmark_receive().await;
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.await;
    }
    let end = st.elapsed().as_secs_f64();
    let req_per_sec = config.requests as f64 / end;
    println!(" {} requests completed in {} seconds", config.requests, end);
    println!(" {} parallel clients", config.num_clients);
    println!(" Latency by percentile distribution:");
    hist.record(end as u64).unwrap();
    println!(" Summary:");
    println!("     Throughput summary: {} requests per second", req_per_sec);
    println!("     Latency summary (msec): ");
    println!("               {}  {}   {}   {}  {}  {}", "avg", "min", "p50", "p95", "p99", "max");
    println!("               {}  {}  {}  {}  {}  {}", 0, hist.min(), hist.value_at_quantile(0.5), hist.value_at_quantile(0.95), hist.value_at_quantile(0.99), hist.max());

    // println!("Count: {}", hist.len());
    // println!("Min: {} s", hist.min());
    // println!("P50: {} s", hist.value_at_quantile(0.5));
    // println!("P95: {} s", hist.value_at_quantile(0.95));
    // println!("P99: {} s", hist.value_at_quantile(0.99));
    // println!("Max: {} s", hist.max());
    // println!("Latency: {:.3}s", end);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(BenchmarkConfig::default());
    benchmark_pipe("ping", config).await?;
    //benchmark_pipe("hset", config).await?;
    Ok(())
}
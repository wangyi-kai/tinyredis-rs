mod cluster;
mod crc;
mod data_structure;
mod db;
mod kvstore;
mod server;
mod parser;
mod connection;
mod listen;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, redis_rs::Error>;

fn main() {}

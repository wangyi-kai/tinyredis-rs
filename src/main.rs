use tokio::sync::{mpsc, oneshot};
use redis_rs::parser::frame::Frame;
use crate::cmd::command::RedisCommand;

mod cluster;
mod crc;
mod data_structure;
mod db;
mod kvstore;
mod object;
mod parser;
mod connection;
mod listen;
mod db_engine;
mod cmd;
mod client;
mod server;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, redis_rs::Error>;
type MpscSender = mpsc::Sender<(oneshot::Sender<Result<Frame>>, RedisCommand)>;
type MpscReceiver = mpsc::Receiver<(oneshot::Sender<Result<Frame>>, RedisCommand)>;

fn main() {}

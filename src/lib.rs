pub mod parser;

pub mod client;
use tokio::sync::{mpsc, oneshot};
use crate::parser::frame::Frame;
use crate::cmd::command::RedisCommand;

mod cluster;
mod crc;
mod data_structure;
mod db;
mod kvstore;
mod object;
mod db_engine;
mod cmd;
pub mod server;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, crate::Error>;
type MpscSender = mpsc::Sender<(oneshot::Sender<Result<Frame>>, RedisCommand)>;
type MpscReceiver = mpsc::Receiver<(oneshot::Sender<Result<Frame>>, RedisCommand)>;

pub const DEFAULT_PORT: u16 = 8000;
pub const DB_SIZE: usize = 256;

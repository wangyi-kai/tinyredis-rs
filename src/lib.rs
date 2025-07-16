pub mod parser;

pub mod client;
use tokio::sync::{mpsc, oneshot};
use crate::parser::frame::Frame;
use parser::cmd::command::RedisCommand;

mod cluster;
mod db;
pub mod server;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
type MpscSender = mpsc::Sender<(oneshot::Sender<Result<Frame>>, RedisCommand)>;
type MpscReceiver = mpsc::Receiver<(oneshot::Sender<Result<Frame>>, RedisCommand)>;

pub const DEFAULT_PORT: u16 = 8000;
pub const DB_SIZE: usize = 256;

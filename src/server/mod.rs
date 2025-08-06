pub mod connection;
pub mod server;
mod shutdown;

use std::sync::{OnceLock};
use crate::server::server::RedisServer;

pub static mut REDIS_SERVER: OnceLock<RedisServer> = OnceLock::new();
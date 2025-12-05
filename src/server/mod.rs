pub mod connection;
pub mod server;
mod shutdown;

use std::sync::{OnceLock};
use crate::config::ServerConfig;
use crate::server::server::RedisServer;

pub static mut REDIS_SERVER: OnceLock<RedisServer> = OnceLock::new();
pub static REDIS_CONFIG: OnceLock<ServerConfig> = OnceLock::new();
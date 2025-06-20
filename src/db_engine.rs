use std::future::Future;
use std::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};

use crate::cmd::command::RedisCommand;
use crate::db::db::RedisDb;

pub struct DbEngine {
    sender: Sender<RedisCommand>,
}

impl DbEngine {
    pub fn new(slot_count_bits: u64, flag: i32, id: i32) -> Self {
        let db = RedisDb::create(slot_count_bits, flag, id);
        let (sender, mut receiver) = std::sync::mpsc::channel::<RedisCommand>();

        loop {
            match receiver.recv() {
                _ => {}
            }
        }

        Self {
            sender,
        }
    }
}
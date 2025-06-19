use std::future::Future;
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::Sender;

use crate::cmd::command::RedisCommand;
use crate::db::db::RedisDb;

pub struct DbEngine {
    sender: Sender<RedisCommand>,
}

impl DbEngine {
    pub fn new(slot_count_bits: u64, flag: i32, id: i32) -> Self {
        let db = RedisDb::create(slot_count_bits, flag, id);
        let (sender, mut receiver) = mpsc::channel::<RedisCommand>(0);

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
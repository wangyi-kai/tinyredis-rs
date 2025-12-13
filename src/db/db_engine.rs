use std::sync::{Arc};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use crate::db::db::RedisDb;
use crate::db::kvstore::iter::KvStoreIterator;
use crate::db::kvstore::KVSTORE_ALLOCATE_DICTS_ON_DEMAND;
use crate::db::object::RedisObject;
use crate::MpscSender;
use crate::persistence::rdb::Rdb;
use crate::server::REDIS_CONFIG;

pub enum DbCommand<V> {
    DbIter(Sender<KvStoreIterator<V>>)
}

#[derive(Debug)]
pub struct DbHandler {
    sender: Vec<MpscSender>,
    pub db_sender: Vec<Sender<DbCommand<RedisObject<String>>>>,
}

impl DbHandler {
    pub fn new(db_num: u32) -> Self {
        let slot_count_bits = 4;
        let flag = KVSTORE_ALLOCATE_DICTS_ON_DEMAND;
        let mut db_list = vec![];
        let mut sender_list = vec![];
        let mut db_sender = vec![];
        for i in 0..db_num {
            let db: RedisDb<RedisObject<String>> = RedisDb::create(slot_count_bits, flag, i as i32);
            sender_list.push(db.sender.clone());
            db_sender.push(db.db_tx.clone());
            db_list.push(db);
        }

        for mut db in db_list {
            tokio::spawn(async move {
                db.run().await
            });
        }
        Self {
            sender: sender_list,
            db_sender,
        }
    }

    pub fn get_sender(&self, idx: usize) -> Option<crate::MpscSender> {
        self.sender.get(idx).cloned()
    }

    pub fn get_size(&self) -> usize {
        self.sender.len()
    }
}
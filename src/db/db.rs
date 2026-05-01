use crate::db::data_structure::dict::dict::{Dict, DictEntry, Value};
use crate::db::kvstore::kvstore::KvStore;
use crate::db::object::{RedisObject, RedisValue};

use std::ptr::NonNull;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug};
use crate::db::db_engine::{RDbCommand};
use crate::db::kvstore::iter::KvStoreIterator;
use crate::parser::cmd::command::{CommandStrategy};

pub enum KeyStatus {
    KeyValid = 0,
    KeyExpire,
    KeyDeleted,
}

pub fn get_key_slot(key: &str) -> usize {
    //key_hash_slot(key)
    0
}

pub struct RedisDb {
    /// The keyspace for this DB. As metadata, holds key sizes histogram
    pub kvs: KvStore,
    /// Timeout of keys with a timeout set
    pub expires: KvStore,
    /// Keys with clients waiting for data (BLPOP)
    pub blocking_keys: Dict,
    /// Keys with clients waiting for data,
    /// and should be unblocked if key is deleted (XREADEDGROUP)
    pub blocking_keys_unblock_on_nokey: Dict,
    /// Blocked keys that received a PUSH
    pub read_keys: Dict,
    /// WATCHED keys for MULTI/EXEC CAS
    pub watched_keys: Dict,
    /// Database ID
    pub id: i32,
    /// Average TTL, just for stats
    pub avg_ttl: i64,
    /// Cursor of the active expire cycle
    pub expires_cursor: u64,
    pub sender: crate::MpscSender,
    pub receiver: crate::MpscReceiver,
    pub db_rx: Receiver<RDbCommand>,
    pub db_tx: Sender<RDbCommand>,
}

impl RedisDb {
    pub fn create(slot_count_bits: u64, flag: i32, id: i32) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(1024);
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        Self {
            kvs: KvStore::create(slot_count_bits, flag),
            expires: KvStore::create(slot_count_bits, flag),
            blocking_keys: Dict::create(),
            blocking_keys_unblock_on_nokey: Dict::create(),
            read_keys: Dict::create(),
            watched_keys: Dict::create(),
            id,
            avg_ttl: 0,
            expires_cursor: 0,
            sender,
            receiver,
            db_rx: rx,
            db_tx: tx,
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                Some((sender, redis_cmd)) = self.receiver.recv() => {
                    debug!("apply command {:?}", redis_cmd);
                    let frame = redis_cmd.apply(self);
                    let _ = sender.send(frame);
                }
                Some(db_cmd) = self.db_rx.recv() => {
                    match db_cmd {
                        RDbCommand::DbIter(sender) => {
                            let iter = self.db_iter();
                            let _ = sender.send(iter);
                        }
                        RDbCommand::RdbData { key, value } => {
                             let _ = self.add(key, value);
                        }
                    }
                }
                else => break,
            }
        }
    }

    pub fn find(&self, key: &RedisObject) -> Option<&mut RedisObject> {
        let k = match &key.ptr {
            RedisValue::String(s) => s,
            _ => return None,
        };
        let de = self.kvs.dict_find(0, k);
        if let Some(mut de) = de {
            unsafe {
                let val = de.as_mut().get_val();
                match val {
                    Value::Val(obj) => Some(obj),
                    _ => None,
                }
            }
        } else {
            None
        }
    }

    pub fn add(&mut self, key: RedisObject, val: RedisObject) -> Option<NonNull<DictEntry>> {
        let key = match key.ptr {
            RedisValue::String(s) => s,
            _ => { "".to_string() }
        };
        let slot = get_key_slot(&key);
        let de = self.kvs.add(slot as i32, key, val);
        de
    }

    pub fn delete(&mut self, key: &RedisObject) {
        let key = match &key.ptr {
            RedisValue::String(s) => s,
            _ => { "" }
        };
        let slot = get_key_slot(&key) as i32;
        let de = self.kvs.dict_delete(slot, key);
        if de.is_some() {
            self.expires.dict_delete(slot, key);
        }
    }

    pub fn set_val(&mut self, key: &RedisObject, val: RedisObject) {
        let old = self.find(key);
        if let Some(old) = old {
            *old = val;
        } else {
            self.add(key.clone(), val);
        }
    }

    pub fn db_size(&self) -> u64 {
        self.kvs.kvstore_size()
    }

    pub fn db_iter(&mut self) -> KvStoreIterator {
        self.kvs.iter()
    }
}



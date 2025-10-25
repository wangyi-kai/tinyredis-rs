use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::kvstore::kvstore::KvStore;
use crate::db::object::{RedisObject, RedisValue};

use std::marker::PhantomData;
use std::ptr::NonNull;

use tokio::sync::mpsc;
use tracing::{debug, info};
use crate::cluster::cluster::key_hash_slot;
use crate::db::kvstore::iter::KvStoreIterator;
use crate::parser::cmd::command::{CommandStrategy, RedisCommand};

pub enum KeyStatus {
    KeyValid = 0,
    KeyExpire,
    KeyDeleted,
}

pub fn get_key_slot(key: &str) -> usize {
    //key_hash_slot(key)
    0
}

pub struct RedisDb<V> {
    /// The keyspace for this DB. As metadata, holds key sizes histogram
    pub kvs: KvStore<V>,
    /// Timeout of keys with a timeout set
    pub expires: KvStore<V>,
    /// Keys with clients waiting for data (BLPOP)
    blocking_keys: Dict<V>,
    /// Keys with clients waiting for data,
    /// and should be unblocked if key is deleted (XREADEDGROUP)
    blocking_keys_unblock_on_nokey: Dict<V>,
    /// Blocked keys that received a PUSH
    read_keys: Dict<V>,
    /// WATCHED keys for MULTI/EXEC CAS
    watched_keys: Dict<V>,
    /// Database ID
    id: i32,
    /// Average TTL, just for stats
    avg_ttl: i64,
    /// Cursor of the active expire cycle
    expires_cursor: u64,
    pub(crate) sender: crate::MpscSender,
    receiver: crate::MpscReceiver,
    _maker: PhantomData<V>,
}

impl<V> RedisDb<V> {
    pub fn create(slot_count_bits: u64, flag: i32, id: i32) -> Self {
        let (sender, receiver) = mpsc::channel(1024);
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
            _maker: PhantomData,
        }
    }

    pub async fn run(&mut self) {
        while let Some((sender, command)) = self.receiver.recv().await {
            debug!("apply command {:?}", command);
            let db = unsafe {
                &mut *(self as *mut RedisDb<V> as *mut RedisDb<RedisObject<String>>)
            };
            let frame = command.apply(db);
            let _ = sender.send(frame);
        }
    }

    pub fn find(&self, key: &RedisObject<String>) -> Option<&mut V> {
        let k = match &key.ptr {
            RedisValue::String(s) => s,
            _ => return None,
        };
        let de = self.kvs.dict_find(0, k);

        if let Some(de) = de {
            unsafe {
                let val = (*de.as_ptr()).get_val();
                Some(val)
            }
        } else {
            None
        }
    }

    pub fn add(&mut self, key: RedisObject<String>, val: V) -> Option<NonNull<DictEntry<V>>> {
        self.add_internal(key, val)
    }

    fn add_internal(&mut self, key: RedisObject<String>, val: V) -> Option<NonNull<DictEntry<V>>> {
        let key = match key.ptr {
            RedisValue::String(s) => s,
            _ => { "".to_string() }
        };
        let slot = get_key_slot(&key);
        let de = self.kvs.add(slot as i32, key, val);
        de
    }

    pub fn delete(&mut self, key: &RedisObject<String>) {
        self.generic_delete(key)
    }

    fn generic_delete(&mut self, key: &RedisObject<String>) {
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

    pub fn set_val(&mut self, key: &RedisObject<String>, val: V) {
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
}



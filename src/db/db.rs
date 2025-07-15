use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::kvstore::kvstore::KvStore;
use crate::object::{RedisObject, RedisValue};

use std::marker::PhantomData;
use std::ptr::NonNull;

use tokio::sync::mpsc;
use crate::parser::cmd::command::{CommandStrategy, RedisCommand};

pub enum KeyStatus {
    KeyValid = 0,
    KeyExpire,
    KeyDeleted,
}

pub struct RedisDb<V> {
    /// The keyspace for this DB. As metadata, holds key sizes histogram
    keys: KvStore<V>,
    /// Timeout of keys with a timeout set
    expires: KvStore<V>,
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
            keys: KvStore::create(slot_count_bits, flag),
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
            let frame = match command {
                RedisCommand::Hash(cmd) => {
                    let db = unsafe {
                        &mut *(self as *mut RedisDb<V> as *mut RedisDb<RedisObject<String>>)
                    };
                    cmd.apply(db)
                }
                _ => Err("Error".into())
            };
            let _ = sender.send(frame);
        }
    }

    pub fn lookup_key(&self, key: &RedisObject<String>) -> Option<&mut V> {
        let k = match &key.ptr {
            RedisValue::String(s) => s,
            _ => return None,
        };

        let de = self.keys.dict_find(0, k);
        unsafe {
            if de.is_none() {
                return None;
            }
            let val = (*de.unwrap().as_ptr()).get_val();
            Some(val)
        }
    }

    pub fn _find(&self, key: &str) -> Option<NonNull<DictEntry<V>>> {
        self.keys.dict_find(0, key)
    }

    pub fn add(&mut self, key: RedisObject<String>, val: V) -> Option<NonNull<DictEntry<V>>> {
        self.add_internal(key, val)
    }

    fn add_internal(&mut self, key: RedisObject<String>, val: V) -> Option<NonNull<DictEntry<V>>> {
        let slot = 0;
        let key = match key.ptr {
            RedisValue::String(s) => s,
            _ => { "".to_string() }
        };
        let de = self.keys.add(slot, key, val);
        de
    }

    fn generic_delete(&mut self, key: &RedisObject<String>) {
        let slot = 0;
        let key = match &key.ptr {
            RedisValue::String(s) => s,
            _ => { "" }
        };
        let de = self.keys.dict_delete(slot, key);
        if de.is_some() {
            self.expires.dict_delete(slot, key);
        }
    }

    fn set_val(&mut self, key: RedisObject<String>, _val: V, _overwrite: i32, mut de: Option<NonNull<DictEntry<V>>>) {
        let slot = 0;
        let key = match key.ptr {
            RedisValue::String(s) => s,
            _ => { "".to_string() }
        };
        if de.is_some() {
            de = self.keys.dict_find(slot, &key);
        }
        unsafe {
            let _old = (*de.unwrap().as_ptr()).get_val();
        }
    }

    pub fn db_size(&self) -> u64 {
        self.keys.kvstore_size()
    }
}

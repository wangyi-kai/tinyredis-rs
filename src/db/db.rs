use std::hash::Hash;
use std::ptr::NonNull;
use crate::dict::dict::{Dict, DictEntry, Value};
use crate::kvstore::kvstore::KvStore;
use crate::server::RedisObject;

pub enum KeyStatus {
    KeyValid = 0,
    KeyExpire,
    KeyDeleted,
}

pub struct RedisDb<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    /// The keyspace for this DB. As metadata, holds key sizes histogram
    keys: KvStore<K, V>,
    /// Timeout of keys with a timeout set
    expires: KvStore<K, V>,
    /// Keys with clients waiting for data (BLPOP)
    blocking_keys: Dict<K, V>,
    /// Keys with clients waiting for data,
    /// and should be unblocked if key is deleted (XREADEDGROUP)
    blocking_keys_unblock_on_nokey: Dict<K, V>,
    /// Blocked keys that received a PUSH
    read_keys: Dict<K, V>,
    /// WATCHED keys for MULTI/EXEC CAS
    watched_keys: Dict<K, V>,
    /// Database ID
    id: i32,
    /// Average TTL, just for stats
    avg_ttl: i64,
    /// Cursor of the active expire cycle
    expires_cursor: u64,
}

impl<K, V> RedisDb<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn find(&self, key: &K) -> Option<NonNull<DictEntry<K, V>>> {
        self.keys.dict_find(0, key)
    }

    pub fn add(&mut self, key: RedisObject, val: RedisObject) -> Option<NonNull<DictEntry<K, V>>> {
        self.add_internal(key, val, 0)
    }

    fn add_internal(&mut self, key: RedisObject, val: RedisObject, update_if_exist: i32) -> Option<NonNull<DictEntry<K, V>>> {
        let slot = 0;
        let de = self.keys.dict_add_raw(slot, key.ptr);
        de
    }

    fn set_val(
        &mut self,
        key: RedisObject,
        val: RedisObject,
        overwrite: i32,
        mut de: Option<NonNull<DictEntry<K, V>>>,
    ) {
        let slot = 0;
        if de.is_some() {
            de = self.keys.dict_find(slot, key.ptr as K);
        }

    }
}
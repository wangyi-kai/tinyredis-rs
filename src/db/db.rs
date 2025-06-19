use crate::data_structure::dict::dict::{Dict, DictEntry, Value};
use crate::data_structure::dict::lib::DictType;
use crate::kvstore::kvstore::KvStore;
use crate::server::RedisObject;
use std::hash::Hash;
use std::ptr::NonNull;
use std::sync::Arc;

pub enum KeyStatus {
    KeyValid = 0,
    KeyExpire,
    KeyDeleted,
}

pub struct RedisDb<K, V>
where
    K: Default + Clone + Eq + Hash,
    V: Default + PartialEq + Clone,
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
where
    K: Default + Clone + Eq + Hash,
    V: Default + PartialEq + Clone,
{
    pub fn create(
        slot_count_bits: u64,
        flag: i32,
        id: i32,
    ) -> Self<K, V> {
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
        }
    }
    pub fn find(&self, key: &K) -> Option<NonNull<DictEntry<K, V>>> {
        self.keys.dict_find(0, key)
    }

    pub fn add(&mut self, key: RedisObject, val: RedisObject) -> Option<NonNull<DictEntry<K, V>>> {
        self.add_internal(key, val, 0)
    }

    fn add_internal(
        &mut self,
        key: RedisObject,
        val: RedisObject,
        update_if_exist: i32,
    ) -> Option<NonNull<DictEntry<K, V>>> {
        let slot = 0;
        let de = self.keys.dict_add_raw(slot, key.ptr);
        de
    }

    fn generic_delete(&mut self, key: &RedisObject) {
        let mut table = 0;
        let slot = 0;
        let de = self.keys.dict_delete(slot, &key.ptr as K);
        if de.is_some() {
            self.expires.dict_delete(slot, &key.ptr as K);
        }
    }

    fn set_val(
        &mut self,
        key: RedisObject,
        val: RedisObject,
        overwrite: i32,
        mut de: Option<NonNull<DictEntry<K, Value>>>,
    ) {
        let slot = 0;
        if de.is_some() {
            de = self.keys.dict_find(slot, key.ptr as K);
        }
        unsafe {
            let old = (*de.unwrap().as_ptr()).get_val();
        }
    }

    pub fn db_size(&self) -> u64 {
        self.keys.kvstore_size()
    }
}

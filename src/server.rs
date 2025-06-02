use crate::dict::dict::Dict;
use crate::kvstore::kvstore::KvStore;

const LRU_BITS: usize = 24;

pub struct RedisDb<K, V> {
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

pub struct RedisObject {
    object_type: usize,
    encoding: usize,
    lru: usize,
    ref_count: i32,
}
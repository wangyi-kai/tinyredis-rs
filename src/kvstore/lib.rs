use std::ptr::NonNull;
use crate::dict::dict::Dict;
use crate::kvstore::kvstore::KvStore;


pub type KvStoreScanShouldSkipDict<K, V> = fn(d: &mut Dict<K, V>) -> usize;
pub type KvStoreExpandShouldSkipDictIndex = fn(didx: usize) -> usize;
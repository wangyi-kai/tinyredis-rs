use crate::data_structure::dict::dict::Dict;

pub type KvStoreScanShouldSkipDict<K, V> = fn(d: &mut Dict<K, V>) -> usize;
pub type KvStoreExpandShouldSkipDictIndex = fn(didx: usize) -> usize;
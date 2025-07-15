use crate::db::data_structure::dict::dict::Dict;

pub type KvStoreScanShouldSkipDict<V> = fn(d: &mut Dict<V>) -> usize;
pub type KvStoreExpandShouldSkipDictIndex = fn(didx: usize) -> usize;

use crate::db::data_structure::dict::dict::Dict;
use crate::db::object::RedisObject;

pub type KvStoreScanShouldSkipDict = fn(d: &mut Dict) -> usize;
pub type KvStoreExpandShouldSkipDictIndex = fn(didx: usize) -> usize;

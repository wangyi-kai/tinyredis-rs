pub mod kvstore;
mod iter;
mod lib;
mod meta;

const MAX_KEYSIZES_BINS: usize = 60;
const MAX_KEYSIZES_TYPES: usize = 5;
const KVSTORE_ALLOCATE_DICTS_ON_DEMAND: i32 = 1 << 0;
const KVSTORE_FREE_EMPTY_DICTS: i32 = 1 << 1;
const KVSTORE_ALLOC_META_KEYS_HIST: i32 = 1 << 2;
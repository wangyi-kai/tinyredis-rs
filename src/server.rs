use std::any::Any;
use std::hash::Hash;

/// A redis object, that is a type able to hold a string / list / set

/// The actual Redis Object
/// String object
const OBJ_STRING: usize = 0;
/// List object
const OBJ_LIST: usize = 1;
/// Set object
const OBJ_SET: usize = 2;
/// Sorted set object
const OBJ_ZSET: usize = 3;
/// Hash object
const OBJ_HASH: usize = 4;
/// Max number of basic object types
const OBJ_TYPE_BASIC_MAX: usize = 5;

const LRU_BITS: usize = 24;

pub struct RedisObject {
    object_type: usize,
    encoding: usize,
    lru: usize,
    ref_count: i32,
    pub ptr: Box<dyn Any>,
}
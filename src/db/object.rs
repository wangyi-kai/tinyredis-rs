use crate::db::data_structure::dict::dict::Dict;
use crate::db::data_structure::intset::intset::IntSet;
use crate::db::data_structure::skiplist::skiplist::SkipList;
use crate::db::data_structure::ziplist::ziplist::ZipList;
use crate::db::data_structure::adlist::adlist::LinkList;

/// A redis object, that is a type able to hold a string / list / set

/// The actual Redis Object
/// String object
const OBJ_STRING: u32 = 0;
/// List object
const OBJ_LIST: u32 = 1;
/// Set object
const OBJ_SET: u32 = 2;
/// Sorted set object
const OBJ_ZSET: u32 = 3;
/// Hash object
const OBJ_HASH: u32 = 4;
/// Max number of basic object types
const OBJ_TYPE_BASIC_MAX: u32 = 5;

/// Raw representation
pub const OBJ_ENCODING_RAW: u32 = 0;
/// Encoded as integer
pub const OBJ_ENCODING_INT: u32 = 1;
/// Encoded as hash table
pub const OBJ_ENCODING_HT: u32 = 2;
/// old hash encoding
pub const OBJ_ENCODING_ZIPMAP: u32 = 3;
///OBJ_ENCODING_LINKEDLIST
pub const OBJ_ENCODING_LINKEDLIST: u32 = 4;
/// list/hash/zset encoding
pub const OBJ_ENCODING_ZIPLIST: u32 = 5;
/// Encoded as intset
pub const OBJ_ENCODING_INTSET: u32 = 6;
/// Encoded as skiplist
pub const OBJ_ENCODING_SKIPLIST: u32 = 7;
/// Embedded sds string encoding
pub const OBJ_ENCODING_EMBSTR: u32 = 8;
/// Encoded as linked list of listpacks
const OBJ_ENCODING_QUICKLIST: u32 = 9;

const LRU_BITS: u32 = 24;
/// Max value of obj->lru
const LRU_CLOCK_MAX: u32 = (1 << LRU_BITS) - 1;

const OBJ_SHARED_REFCOUNT: i32 = i32::MAX;
const OBJ_STATIC_REFCOUNT: i32 = i32::MAX - 1;
const OBJ_FIRST_SPECIAL_REFCOUNT: i32 = OBJ_STATIC_REFCOUNT;

#[derive(Clone)]
pub enum RedisValue<T> {
    String(T),
    List(ListObject<T>),
    Hash(Dict<T>),
    SortSet(SkipList),
    Set(IntSet),
}

#[derive(Clone)]
pub enum ListObject<T> {
    LinkList(LinkList<T>),
    ZipList(ZipList),
}

#[derive(Clone)]
pub struct RedisObject<T> {
    /// object type
    object_type: u32,
    /// object encoding
    pub encoding: u32,
    /// object last visit time
    lru: u32,
    /// object reference count
    ref_count: i32,
    /// actual object
    pub ptr: RedisValue<T>,
}

#[allow(dead_code)]
impl<T> RedisObject<T> {
    fn create(object_type: u32, ptr: RedisValue<T>) -> Self {
        Self {
            object_type,
            encoding: OBJ_ENCODING_RAW,
            lru: 0,
            ref_count: 1,
            ptr,
        }
    }

    fn create_raw_string_object(s: String) -> RedisObject<String> {
        //let s_object = Box::new(s);
        let s_object = RedisValue::String(s);
        RedisObject::create(OBJ_STRING, s_object)
    }

    pub fn create_string_object(s: String) -> RedisObject<String> {
        RedisObject::<T>::create_raw_string_object(s)
    }

    // pub fn create_quicklist_object(fill: i32, compress: i32) -> Self {
    //     let l = QuickList::new(fill, compress);
    //     let mut o = RedisObject::create(OBJ_LIST, Box::new(l));
    //     o.encoding = OBJ_ENCODING_QUICKLIST;
    //     o
    // }

    // pub fn create_set_object() -> Self {
    //     let d = Dict::create();
    //     let mut o = RedisObject::create(OBJ_SET, RedisValue::S);
    //     o.encoding = OBJ_ENCODING_HT;
    //     o
    // }

    pub fn create_hash_object() -> Self {
        let ht = Dict::create();
        let mut o = RedisObject::create(OBJ_HASH, RedisValue::Hash(ht));
        o.encoding = OBJ_ENCODING_HT;
        o
    }

    pub fn create_intset_object() -> Self {
        let is = IntSet::new();
        let mut o = RedisObject::create(OBJ_SET, RedisValue::Set(is));
        o.encoding = OBJ_ENCODING_INTSET;
        o
    }

    pub fn create_list_object() -> Self {
        let z = ZipList::new();
        let mut o = RedisObject::create(OBJ_LIST, RedisValue::List(ListObject::ZipList(z)));
        o.encoding = OBJ_ENCODING_HT;
        o
    }

    pub fn create_skiplist_object() -> Self {
        let s = SkipList::new();
        let mut o = RedisObject::create(OBJ_ZSET, RedisValue::SortSet(s));
        o.encoding = OBJ_ENCODING_SKIPLIST;
        o
    }

    pub fn incr_ref_count(&mut self) {
        if self.ref_count > OBJ_FIRST_SPECIAL_REFCOUNT {
            self.ref_count += 1;
        } else {
            if self.ref_count == OBJ_SHARED_REFCOUNT {
            } else if self.ref_count == OBJ_STATIC_REFCOUNT {
                panic!("You tried to retain an object allocated in the stack");
            }
        }
    }

    pub fn decr_ref_count(&mut self) {
        if self.ref_count == OBJ_SHARED_REFCOUNT {
            return;
        }
        if self.ref_count < 0 {
            panic!(
                "illegal decrRefCount for object with: type {}, encoding {}, refcount {}",
                self.object_type, self.encoding, self.ref_count
            );
        }

        self.ref_count -= 1;
    }
}


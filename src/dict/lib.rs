use std::hash::Hash;
use rand::Rng;
use std::ffi::c_void;
use crate::dict::dict::{Dict, DictEntry};
use crate::dict::lib::DictResizeFlag::DictResizeEnable;

pub(crate) const DICT_HT_INITIAL_EXP: usize = 2;
pub(crate) const DICT_HT_INITIAL_SIZE: usize = 1 << DICT_HT_INITIAL_EXP;
pub const DICT_FORCE_RESIZE_RATIO: u64 = 4;
pub static mut DICT_CAN_RESIZE: DictResizeFlag = DictResizeEnable;
pub(crate) const HASHTABLE_MIN_FILL: u64 = 8;
pub(crate) const LONG_MAX: u64 = 0x7FFF_FFFF_FFFF_FFFF;
pub(crate) const DICT_STATS_VECTLEN: usize = 50;
pub const GETFAIR_NUM_ENTRIES: usize = 15;

#[derive(PartialEq)]
pub enum DictResizeFlag {
    DictResizeEnable,
    DictResizeAvoid,
    DictResizeForbid,
}

pub type DictScanFunction<K, V> = fn(de: &mut DictEntry<K, V>);
pub type DictDefragAllocFunction = fn(ptr: *mut c_void);

pub struct DictType<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub hash_function: Option<Box<dyn Fn(&K) -> u64>>,
    pub rehashing_started: Option<Box<dyn Fn(&Dict<K, V>)>>,
    pub rehashing_completed: Option<Box<dyn Fn(&Dict<K, V>)>>,
    pub dict_meta_data_bytes: Option<Box<dyn Fn(&Dict<K, V>) -> usize>>,
    //pub user_data: Option<KvStore<K, V>>,
}

#[inline]
pub fn dict_size(exp: i32) -> u64 {
    return if exp == -1 {
        0
    } else {
        1 << exp
    }
}

#[inline]
pub fn dict_size_mask(exp: i32) -> u64 {
    return if exp == -1 {
        0
    } else {
        dict_size(exp) - 1
    }
}

pub fn entry_mem_usage<K, V> ()-> usize
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    size_of::<DictEntry<K, V>>()
}

pub fn next_exp(size: usize) -> i32 {
    if size <= DICT_HT_INITIAL_SIZE {
        return DICT_HT_INITIAL_EXP as i32;
    }
    let long_bits = size_of::<usize>() * 8;
    if size >= LONG_MAX as usize {
        return (long_bits - 1) as i32;
    }
    let leading_zeros = (size - 1).leading_zeros() as usize;
    (long_bits - leading_zeros) as i32
}

pub fn dict_set_resize_enabled(enable: DictResizeFlag) {
    unsafe {
        DICT_CAN_RESIZE = enable
    }
}

pub fn random_ulong() -> u64 {
    rand::rng().random::<u64>()
}

pub fn random_u32() -> u32 {
    rand::rng().random::<u32>()
}

pub fn random_i32() -> i32 {
    rand::rng().random::<i32>()
}
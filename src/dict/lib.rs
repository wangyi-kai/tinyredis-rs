use std::hash::Hash;
use crate::dict::dict::Dict;
use crate::dict::lib::DictResizeFlag::DictResizeEnable;

pub(crate) const DICT_HT_INITIAL_EXP: usize = 2;
pub(crate) const DICT_HT_INITIAL_SIZE: usize = 1 << DICT_HT_INITIAL_EXP;
pub const DICT_FORCE_RESIZE_RATIO: u64 = 4;
pub static mut DICT_CAN_RESIZE: DictResizeFlag = DictResizeEnable;
pub(crate) const HASHTABLE_MIN_FILL: u64 = 8;
pub(crate) const LONG_MAX: u64 = 0x7FFF_FFFF_FFFF_FFFF;

#[derive(PartialEq)]
pub enum DictResizeFlag {
    DictResizeEnable,
    DictResizeAvoid,
    DictResizeForbid,
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
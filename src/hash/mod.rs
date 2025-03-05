pub mod dict;
pub mod error;
pub mod hash;
mod hash_iter;

const DICT_HT_INITIAL_EXP: usize = 2;
const DICT_HT_INITIAL_SIZE: usize = 1 << DICT_HT_INITIAL_EXP;
const DICT_FORCE_RESIZE_RATIO: u64 = 4;
const DICT_CAN_RESIZE: DictResizeFlag = DictResizeFlag::DictResizeEnable;
const HASHTABLE_MIN_FILL: u64 = 8;
const LONG_MAX: u64 = 0x7FFF_FFFF_FFFF_FFFF;

#[derive(PartialEq)]
pub enum DictResizeFlag {
    DictResizeEnable,
    DictResizeAvoid,
    DictResizeForbid,
}

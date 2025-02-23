pub mod dict;
pub mod error;
pub mod hash;

const DICT_HT_INITIAL_EXP: usize = 2;
const DICT_HT_INITIAL_SIZE: usize = 1 << DICT_HT_INITIAL_EXP;
const DICT_FORCE_RESIZE_RATIO: u64 = 4;
const DICT_CAN_RESIZE: DictResizeFlag = DictResizeFlag::DictResizeEnable;
const HASHTABLE_MIN_FILL: u64 = 8;

pub enum DictResizeFlag {
    DictResizeEnable,
    DictResizeAvoid,
    DictResizeForbid,
}

impl PartialEq for DictResizeFlag {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DictResizeFlag::DictResizeEnable, DictResizeFlag::DictResizeEnable) => self == other,
            (DictResizeFlag::DictResizeAvoid, DictResizeFlag::DictResizeAvoid) => self == other,
            (DictResizeFlag::DictResizeForbid, DictResizeFlag::DictResizeForbid) => self == other,
            _ => false,
        }
    }
}
use crate::db::data_structure::intset::*;

pub fn intset_value_encoding(v: i64) -> u8 {
    if v < INT32_MIN || v > INT32_MAX {
        INTSET_ENC_INT64
    } else if v < INT16_MIN || v > INT16_MAX {
        INTSET_ENC_INT32
    } else {
        INTSET_ENC_INT16
    }
}

use crate::skiplist::skiplist::SkipList;

mod skiplist;
mod test;

const SKIP_LIST_MAX_LEVEL: i32 = 32;
const SKIP_LIST_P: f32 = 0.25;
const SKIP_MAX_SEARCH: usize = 10;
const RAND_MAX: i32 = i32::MAX;

#[derive(Copy, Clone)]
pub struct RangeSpec {
    min: i64,
    max: i64,
    min_ex: i32,
    max_ex: i32,
}

pub fn value_gte_min(value: i64, range_spec: RangeSpec) -> bool {
    if range_spec.min_ex == 1 {
        value > range_spec.min
    } else {
        value >= range_spec.min
    }
}

pub fn value_lte_max(value: i64, range_spec: RangeSpec) -> bool {
    if range_spec.max_ex == 1 {
        value < range_spec.max
    } else {
        value <= range_spec.max
    }
}

pub fn is_in_range(zsl: SkipList, range_spec: RangeSpec) -> bool {
    if range_spec.min > range_spec.max || (range_spec.min == range_spec.max && (range_spec.min_ex != 0 || range_spec.max_ex != 0)) {
        return false;
    }
    let x = zsl.tail;
    unsafe {
        if let Some(x) = x {
            if !value_gte_min((*x.as_ptr()).get_score(), range_spec) {
                return false;
            }
        } else {
            return false;
        }
        let x = (*zsl.head.unwrap().as_ptr()).level[0].forward;
        if let Some(x) = x {
            if !value_lte_max((*x.as_ptr()).get_score(), range_spec) {
                return false;
            }
        } else {
            return false;
        }
        true
    }
}


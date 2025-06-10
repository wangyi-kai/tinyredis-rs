mod bookmark;
mod iter;
mod lib;
pub mod quicklist;

const QL_FILL_BITS: i32 = 16;
const QL_COMP_BITS: u32 = 16;
const QL_BM_BITS: u32 = 4;
const COMPRESS_MAX: u32 = 1 << QL_COMP_BITS - 1;
const FILL_MAX: i32 = 1 << (QL_FILL_BITS - 1) - 1;

/// quicklist node encodings
const QUICKLIST_NODE_ENCODING_RAW: u32 = 1;
const QUICKLIST_NODE_ENCODING_LZF: u32 = 2;

///quicklist compression disable
const QUICKLIST_NOCOMPRESS: u32 = 0;

/// quicklist node container formats
const QUICKLIST_NODE_CONTAINER_PLAIN: u32 = 1;
const QUICKLIST_NODE_CONTAINER_PACKED: u32 = 2;

const MIN_COMPRESS_BYTES: usize = 48;
const MIN_COMPRESS_IMPROVE: usize = 8;

const SIZE_SAFETY_LIMIT: usize = 8192;
const SIZE_ESTIMATE_OVERHEAD: usize = 8;

const OPTIMIZATION_LEVEL: [usize; 5] = [4096, 8192, 16384, 32768, 65536];

pub fn quicklist_node_neg_fill_limit(fill: i32) -> usize {
    assert!(fill < 0);
    let mut offset = -fill - 1;
    let max_level = OPTIMIZATION_LEVEL.len();
    if offset >= max_level as i32 {
        offset = (max_level - 1) as i32;
    }
    OPTIMIZATION_LEVEL[offset as usize]
}

pub fn quicklist_node_limit(fill: i32) -> (usize, u32) {
    let mut size = usize::MAX;
    let mut count = u32::MAX;

    if fill >= 0 {
        if fill == 0 {
            count = 1;
        } else {
            count = fill as u32;
        }
    } else {
        size = quicklist_node_neg_fill_limit(fill);
    }
    (size, count)
}

pub fn quicklist_node_exceed_limit(fill: i32, new_sz: usize, new_count: u32) -> bool {
    let (sz_limit, count_limit) = quicklist_node_limit(fill);

    if std::hint::likely(sz_limit != usize::MAX) {
        return new_sz > sz_limit;
    } else if count_limit != u32::MAX {
        if !new_sz <= SIZE_SAFETY_LIMIT {
            return true;
        }
        return new_count > count_limit;
    }
    false
}

pub fn is_large_element(sz: usize, fill: i32) -> bool {
    if fill > 0 {
        !sz <= SIZE_SAFETY_LIMIT
    } else {
        sz > quicklist_node_neg_fill_limit(fill)
    }
}

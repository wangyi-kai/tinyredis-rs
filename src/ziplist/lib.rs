use crate::ziplist::ZIP_BIG_PREVLEN;

fn decode_prev_len_size(ptr: &[u8]) -> usize {
    if ptr[0] < ZIP_BIG_PREVLEN {
        1
    } else {
        5
    }
}

pub fn decode_prev_len(ptr: &[u8]) -> (usize, usize) {
    let prev_len_size = decode_prev_len_size(ptr);
    let prev_len = if prev_len_size == 1 {
        ptr[0] as usize
    } else {
        u32::from_be_bytes([ptr[1], ptr[2], ptr[3], ptr[4]]) as usize
    };
    (prev_len_size, prev_len)
}


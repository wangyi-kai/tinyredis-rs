use crate::ziplist::{*};
use crate::ziplist::error::ZipListError;
use crate::ziplist::ziplist::ZlEntry;

pub fn decode_prev_len_size(ptr: &[u8]) -> usize {
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

pub fn entry_encoding(ptr: &[u8]) -> u8 {
    let mut encoding = ptr[0];
    if encoding < ZIP_STR_MASK {
        encoding &= ZIP_STR_MASK;
    }
    encoding
}

pub fn decode_length(ptr: &[u8], encoding: u8) -> (usize, usize) {
    if encoding < ZIP_STR_MASK {
        match encoding {
            ZIP_STR_06B => {
                (1, (ptr[0] & 0x3f) as usize)
            }
            ZIP_STR_14B => {
                let len = (((ptr[0] & 0x3f) as usize) << 8) | (ptr[1] as usize);
                (2, len)
            }
            ZIP_STR_32B => {
                let len = u32::from_be_bytes([ptr[1], ptr[2], ptr[3], ptr[4]]) as usize;
                (5, len)
            }
            _ => (0, 0), // bad encoding
        }
    } else {
        let len_size = 1;
        let len = match encoding {
            ZIP_INT_8B => 1,
            ZIP_INT_16B => 2,
            ZIP_INT_24B => 3,
            ZIP_INT_32B => 4,
            ZIP_INT_64B => 8,
            imm if imm >= ZIP_INT_IMM_MIN && imm <= ZIP_INT_IMM_MAX => 0,
            _ => return (0, 0), // bad encoding
        };
        (len_size, len)
    }
}

#[inline]
pub fn encoding_len_size(encoding: u8) -> usize {
    match encoding {
        ZIP_INT_8B | ZIP_INT_16B | ZIP_INT_24B | ZIP_INT_32B | ZIP_INT_64B => 1,
        ZIP_INT_IMM_MIN..=ZIP_INT_IMM_MAX => 1,
        ZIP_STR_06B => 1,
        ZIP_STR_14B => 2,
        ZIP_STR_32B => 5,
        _ => ZIP_ENCODING_SIZE_INVALID as usize,
    }
}

fn string_to_int(s: &str) -> Result<i64, ZipListError> {
    let b = s.as_bytes();
    let s_len = b.len();
    let mut negative = false;
    let mut v: u64 = 0;

    if s_len == 0 || s_len >= LONG_STR_SIZE {
        return Err(ZipListError::InValidString);
    }
    if s_len == 1 && b[0] == b'0' {
        return Ok(0);
    }

    let mut index = 0;
    let mut p_len = 0;

    if b[0] == b'-' {
        negative = true;
        index += 1;
        p_len += 1;
    }
    if b[0] >= b'1' && b[0] <= b'9' {
        v = (b[0] - b'0') as u64;

    } else {
        return Err(ZipListError::InvalidFirstDigit);
    }

}

pub fn try_encoding(entry: &str, entry_len: usize) {
    if entry_len == 0 {
        return;
    }

}


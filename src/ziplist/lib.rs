use std::cmp;
use std::str::from_utf8;
use crate::ziplist::{*};
use crate::ziplist::error::ZipListError;
use crate::ziplist::ziplist::{ZipList};

#[derive(Debug)]
pub enum Content {
    Char(String),
    Integer(i64),
}

pub fn decode_prev_len_size(ptr: &[u8]) -> u32 {
    if ptr[0] < ZIP_BIG_PREVLEN {
        1
    } else {
        5
    }
}

pub fn decode_prev_len(ptr: &[u8]) -> (u32, u32) {
    let prev_len_size = decode_prev_len_size(ptr);
    let prev_len = if prev_len_size == 1 {
        ptr[0] as u32
    } else {
        u32::from_le_bytes([ptr[1], ptr[2], ptr[3], ptr[4]])
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


/* Decode the entry encoding type and data length (string length for strings,
 * number of bytes used for the integer for integer entries) encoded in 'ptr'.
 * The 'encoding' variable is input, extracted by the caller, the 'lensize'
 * variable will hold the number of bytes required to encode the entry
 * length, and the 'len' variable will hold the entry length.
 * On invalid encoding error, lensize is set to 0. */
pub fn decode_length(ptr: &[u8], encoding: u8) -> (u32, u32) {
    if encoding < ZIP_STR_MASK {
        match encoding {
            ZIP_STR_06B => {
                (1, (ptr[0] & 0x3f) as u32)
            }
            ZIP_STR_14B => {
                let len = (((ptr[0] & 0x3f) as u32) << 8) | (ptr[1] as u32);
                (2, len)
            }
            ZIP_STR_32B => {
                let len = (ptr[1] as u32) << 24 | (ptr[2] as u32) <<16 | (ptr[3] as u32) << 8 | (ptr[4] as u32);
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
pub fn encoding_len_size(encoding: u8) -> u32 {
    match encoding {
        ZIP_INT_8B | ZIP_INT_16B | ZIP_INT_24B | ZIP_INT_32B | ZIP_INT_64B => 1,
        ZIP_INT_IMM_MIN..=ZIP_INT_IMM_MAX => 1,
        ZIP_STR_06B => 1,
        ZIP_STR_14B => 2,
        ZIP_STR_32B => 5,
        _ => ZIP_ENCODING_SIZE_INVALID as u32,
    }
}

fn string_to_number(s: &str) -> Result<i64, ZipListError> {
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

    let mut p_len = 0;
    if b[p_len] == b'-' {
        negative = true;
        p_len += 1;
    }
    if b[p_len] >= b'1' && b[p_len] <= b'9' {
        v = (b[p_len] - b'0') as u64;
        p_len += 1;
    } else {
        return Err(ZipListError::InvalidFirstDigit);
    }

    while p_len < s_len {
        let ch = b[p_len];
        if ch < b'0' || ch > b'9' {
            return Err(ZipListError::InvalidChar);
        }
        let digit = (ch - b'0') as u64;
        if v > u64::MAX / 10 {
            return Err(ZipListError::OverFlowMul);
        }
        v *= 10;
        if v > u64::MAX - digit {
            return Err(ZipListError::OverFlowAdd)
        }
        v += digit;
        p_len += 1;
    }

    if negative {
        if v > ((-(i64::MIN + 1) as u64) + 1) {
            return Err(ZipListError::OverFlowNegative);
        }
        Ok(-(v as i64))
    } else {
        if v > i64::MAX as u64 {
            return Err(ZipListError::OverFlowPositive);
        }
        Ok(v as i64)
    }
}

/// Return the integer value and its encoding
pub fn try_encoding(entry: &str) -> Option<(i64, u8)> {
    let len = entry.len();
    if len == 0 || len >= 32 {
        return None;
    }
    match string_to_number(entry) {
        Ok(value) => {
            let encoding = if value >= 0 && value < 12 {
                ZIP_INT_IMM_MIN + value as u8
            } else if value >= i8::MIN as i64 && value <= i8::MAX as i64 {
                ZIP_INT_8B
            } else if value >= i16::MIN as i64 && value <= i16::MAX as i64 {
                ZIP_INT_16B
            } else if value >= INT_24_MIN && value <= INT_24_MAX {
                ZIP_INT_24B
            } else if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
                ZIP_INT_32B
            } else {
                ZIP_INT_64B
            };
            Some((value, encoding))
        }
        Err(_) => {
            None
        }
    }
}

#[inline]
pub fn int_size(encoding: u8) -> u32 {
    match encoding {
        ZIP_INT_8B => 1,
        ZIP_INT_16B => 2,
        ZIP_INT_24B => 3,
        ZIP_INT_32B => 4,
        ZIP_INT_64B => 8,
        n if n >= ZIP_INT_IMM_MIN && n <= ZIP_INT_IMM_MAX => 0,
        _ => 0,
    }
    //panic!("unreachable code reached");
}

pub fn store_prev_entry_length_large(data: Option<&mut [u8]>, len: u32) -> u32 {
    if let Some(p) = data {
        (*p)[0] = ZIP_BIG_PREVLEN;
        (*p)[1..5].copy_from_slice(&len.to_le_bytes());
    }
    (1 + size_of::<u32>()) as u32
}

pub fn store_prev_entry_length(data: Option<&mut [u8]>, len: u32) -> u32 {
    if let Some(p) = data {
        if len < ZIP_BIG_PREVLEN as u32 {
            p[0] = len as u8;
            1
        } else {
            store_prev_entry_length_large(Some(p), len)
        }
    } else if len < ZIP_BIG_PREVLEN as u32 {
        1
    } else {
        (1 + size_of::<u32>()) as u32
    }
}

pub fn is_string(encoding: u8) -> bool {
    encoding & ZIP_STR_MASK < ZIP_STR_MASK
}

pub fn store_entry_encoding(data: Option<&mut [u8]>, encoding: u8, raw_len: u32) -> u32 {
    let mut len = 1;
    let mut buf = [0u8; 5];

    if is_string(encoding) {
        if raw_len <= 0x3f {
            if data.is_none() {
                return len;
            }
            buf[0] = ZIP_STR_06B | (raw_len as u8);
        } else if raw_len <= 0x3fff {
            len += 1;
            if data.is_none() {
                return len;
            }
            buf[0] = ZIP_STR_14B | ((raw_len >> 8) as u8) & 0x3f;
            buf[1] = (raw_len & 0xff) as u8;
        } else {
            len += 4;
            if data.is_none() {
                return len;
            }
            buf[0] = ZIP_STR_32B;
            buf[1] = (raw_len >> 24 & 0xff) as u8;
            buf[2] = (raw_len >> 16 & 0xff) as u8;
            buf[3] = (raw_len >> 8 & 0xff) as u8;
            buf[4] = (raw_len & 0xff) as u8;
        }
    } else {
        if data.is_none() {
            return len;
        }
        buf[0] = encoding;
    }
    if let Some(data) = data {
        data[0..len as usize].copy_from_slice(&buf[0..len as usize]);
    }
    len
}

pub fn prev_len_bytes_diff(ptr: &[u8], len: u32) -> i32 {
    let prev_len_size = decode_prev_len_size(ptr);
    store_prev_entry_length(None, len) as i32 - prev_len_size as i32
}

pub fn save_integer(ptr: &mut [u8], value: i64, encoding: u8) {
    match encoding {
        ZIP_INT_8B => {
            ptr[0] = value as i8 as u8;
        }
        ZIP_INT_16B => {
            let i16 = (value as i16).to_le_bytes();
            ptr[..2].copy_from_slice(&i16);
        }
        ZIP_INT_24B => {
            let i32 = (((value as u64) << 8) as i32).to_le_bytes();
            ptr[..3].copy_from_slice(&i32[1..]);
        }
        ZIP_INT_32B => {
            let i32 = (value as i32).to_le_bytes();
            ptr[..4].copy_from_slice(&i32);
        }
        ZIP_INT_64B => {
            let i64 = value.to_le_bytes();
            ptr[..8].copy_from_slice(&i64);
        }
        imm if imm >= ZIP_INT_IMM_MIN && imm <= ZIP_INT_IMM_MAX => { }
        _ => { panic!("Invalid zip integer encoding"); }
    }
}

pub fn load_integer(ptr: &[u8], encoding: u8) -> i64 {
    match encoding {
        ZIP_INT_8B => {
            ptr[0] as i8 as i64
        }
        ZIP_INT_16B => {
            let bytes = ptr[..2].try_into().unwrap();
            i16::from_le_bytes(bytes) as i64
        }
        ZIP_INT_24B => {
            let mut bytes = [0u8; 4];
            bytes[1..].copy_from_slice(&ptr[..3]);
            i32::from_le_bytes(bytes) as i64 >> 8
        }
        ZIP_INT_32B => {
            let bytes = ptr[..4].try_into().unwrap();
            i32::from_le_bytes(bytes) as i64
        }
        ZIP_INT_64B => {
            let bytes = ptr[..8].try_into().unwrap();
            i64::from_le_bytes(bytes)
        }
        encode if encode >= ZIP_INT_IMM_MIN && encode <= ZIP_INT_IMM_MAX => {
            ((encoding & ZIP_INT_IMM_MASK) as i64) - 1
        }
        _ => {
            panic!("Invalid encoding!")
        }
    }
}

pub fn incr_length(ptr: &mut [u8], incr: usize) {
    let len = u16::from_le_bytes(ptr[ZIPLIST_LENGTH_OFFSET..ZIPLIST_LENGTH_OFFSET + 2].try_into().unwrap());
    if len < u16::MAX {
        ptr[ZIPLIST_LENGTH_OFFSET..ZIPLIST_LENGTH_OFFSET + 2].copy_from_slice(&(len as usize + incr).to_le_bytes())
    }
}

pub fn ziplist_repr(zl: &mut ZipList) {
    let mut pos = 0;
    let mut index = 0;
    let zl_bytes = zl.ziplist_len();
    let num = zl.entry_num();
    let tail_offset = zl.tail_offset();
    println!("total bytes: {}, num entries: {}, tail_offset: {}", zl_bytes, num, tail_offset);
    pos = ZIPLIST_HEADER_SIZE as usize;

    while zl.data[pos] != ZIP_END {
        let entry = match zl.entry_safe(zl_bytes, pos, 1) {
            Ok(entry) => { entry }
            Err(_) => { return; }
        };
        //println!("addr: {}, index: {}, offset: {}, hdr+entry len: {}, hdr len: {}, prevrawlen: {}, prevrawlensize: {}, payload: {}", pos, index, pos, entry.head_size+entry.len, entry.head_size, entry.prev_raw_len, entry.prev_raw_len_size, entry.len);
        for i in 0..(entry.head_size+entry.len) {
            //print!("{:02x}|", zl.data[pos]);
        }
        //print!("\n");
        pos += entry.head_size as usize;
        if is_string(entry.encoding) {
            let s = from_utf8(&zl.data[pos..pos + entry.len as usize]).unwrap();
            println!("[str]: {}", s);
        } else {
            let value = load_integer(&zl.data[pos..], entry.encoding);
            println!("[int]: {}", value);
        }
        pos += entry.len as usize;
        index += 1;
    }
    println!("End");
}

type ZiplistValidateEntryCb = fn(pos: usize, head_count: u32, user_dara: *mut c_void) -> i32;
use std::ffi::c_void;

pub fn ziplist_valid_integerity(zl: &mut ZipList, size: usize, deep: i32, entry_cb: Option<ZiplistValidateEntryCb>, user_data: Option<*mut c_void>) -> i32 {
    if size < (ZIPLIST_HEADER_SIZE + ZIPLIST_END_SIZE) as usize {
        return 0;
    }
    let bytes = zl.ziplist_len();
    if bytes != size {
        return 0;
    }
    if zl.data[size - ZIPLIST_END_SIZE as usize] != ZIP_END {
        return 0;
    }
    if zl.tail_offset() > (size - ZIPLIST_END_SIZE as usize) {
        return 0;
    }
    if deep == 0 {
        return 1;
    }

    let mut count = 0;
    let header_count = zl.entry_num();
    let mut pos = ZIPLIST_HEADER_SIZE as usize;
    let mut prev_raw_size = 0;
    let mut prev = 0;
    while zl.data[pos] != ZIP_END {
        let e = match zl.entry_safe(size, pos, 1) {
            Ok(entry) => entry,
            Err(_) => return 0,
        };
        if e.prev_raw_len != prev_raw_size {
            return 0;
        }
        if let Some(cb) = entry_cb {
            if cb(pos, header_count, user_data.unwrap()) != 0 {
            return 0;
        }
        }
        prev_raw_size = e.head_size + e.len;
        prev = pos;
        pos += (e.head_size + e.len) as usize;
        count += 1;
    }
    if pos != bytes - ZIPLIST_END_SIZE as usize {
        return 0;
    }
    if prev != 0 && prev != zl.tail_offset() {
        return 0;
    }
    if header_count != u16::MAX as u32 && count != header_count {
        return 0;
    }
    1
}

pub fn ziplist_merge(first: &mut Option<ZipList>, second: &mut Option<ZipList>) -> Option<ZipList> {
    let (Some(f), Some(s)) = (first.as_mut(), second.as_mut()) else {
        return None;
    };

    let first_bytes = f.ziplist_len();
    let first_len = f.entry_num();
    let second_bytes = s.ziplist_len();
    let second_len = s.entry_num();
    let first_offset = f.tail_offset();
    let second_offset = s.tail_offset();

    let (mut target, source, append) = if first_len >= second_len {
        (first.take().unwrap(), s, true)
    } else {
        (second.take().unwrap(), f, false)
    };

    let target_bytes = target.ziplist_len();
    let source_bytes = source.ziplist_len();

    let zl_bytes = first_bytes as u32 + second_bytes as u32 - ZIPLIST_HEADER_SIZE - ZIPLIST_END_SIZE;
    let zl_len = cmp::min(first_len + second_len, u16::MAX as u32) as u16;
    assert!(zl_bytes < u32::MAX);

    target.resize(zl_bytes);
    if append {
        target.data[target_bytes - ZIPLIST_END_SIZE as usize..].copy_from_slice(&source.data[ZIPLIST_HEADER_SIZE as usize..source_bytes]);
    } else {
        target.data.copy_within(ZIPLIST_HEADER_SIZE as usize..source_bytes, source_bytes - ZIPLIST_END_SIZE as usize);
        target.data[..source_bytes - ZIPLIST_END_SIZE as usize].copy_from_slice(&source.data[..source_bytes - ZIPLIST_END_SIZE as usize]);
    }

    target.data[..4].copy_from_slice(&zl_bytes.to_le_bytes());
    target.data[8..10].copy_from_slice(&zl_len.to_le_bytes());
    target.data[4..8].copy_from_slice(&(first_bytes as u32 - ZIPLIST_END_SIZE + second_offset as u32 - ZIPLIST_HEADER_SIZE).to_le_bytes());
    target.cascade_update(first_offset);

    if append {

    }
    Some(target)
}

#[cfg(test)]
mod test {
    use crate::ziplist::lib::string_to_number;

    #[test]
    fn to_number() {
        let s = "-1234567899999999".to_string();
        let n = string_to_number(&s);
        match n {
            Ok(n) => {
                println!("number: {}", n);
            }
            Err(e) => {
                println!("Err: {:?}", e);
            }
        }
    }
}
use crate::crc::crc16::crc16;
#[inline]
pub fn key_hash_slot(key: &str) -> u32 {
    let key_vec = key.as_bytes();
    let key_len = key_vec.len();
    let mut s = 0;
    while s < key_len {
        if key_vec[s] == b'{' {
            break;
        }
        s += 1;
    }
    if s == key_len {
        return (crc16(key_vec) & 0x3FFF) as u32;
    }
    let mut e = s + 1;
    while e < key_len {
        if key_vec[e] == b'}' {
            break;
        }
        e += 1;
    }
    if e == key_len || e == s + 1 {
        return (crc16(key_vec) & 0x3FFF) as u32;
    }
    (crc16(&key_vec[s + 1..e]) & 0x3FFF) as u32
}

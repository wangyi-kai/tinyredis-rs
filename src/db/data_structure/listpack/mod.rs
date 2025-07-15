mod listpack;

const LP_HDR_SIZE: usize = 6;
const LP_HDR_NUMELE_UNKNOWN: u16 = u16::MAX;
const LP_MAX_INT_ENCODING_LEN: usize = 9;
const LP_MAX_BACKLEN_SIZE: usize = 5;
const LP_ENCODING_INT: usize = 0;
const LP_ENCODING_STRING: usize = 0;

pub fn lp_set_total_bytes(p: &mut [u8], v: u32) {
    let bytes = v.to_le_bytes();
    p[..4].copy_from_slice(&bytes);
}

pub fn lp_set_num_elements(p: &mut [u8], v: u32) {
    let bytes = v.to_le_bytes();
    p[4] = bytes[0];
    p[5] = bytes[1];
}

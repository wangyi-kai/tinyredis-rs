mod ziplist;
mod lib;

const ZIPLIST_HEADER_SIZE: usize = 10;
const ZIPLIST_END_SIZE: usize = 1;
const ZIP_END: u8 = 255;
const ZIP_BIG_PREVLEN: u8 = 254;

pub const ZIPLIST_HEAD: i32 = 0;
pub const ZIPLIST_TAIL: i32 = 1;

pub const ZIP_STR_MASK: u8 = 0xc0;
pub const ZIP_INT_MASK: u8 = 0x30;

/// string encode
pub const ZIP_STR_06B: u8 = 0 << 6;
pub const ZIP_STR_14B: u8 = 1 << 6;
pub const ZIP_STR_32B: u8 = 2 << 6;

/// integer encode
pub const ZIP_INT_16B: u8 = 0xc0 | (0 << 4);
pub const ZIP_INT_32B: u8 = 0xc0 | (1 << 4);
pub const ZIP_INT_64B: u8 = 0xc0 | (2 << 4);
pub const ZIP_INT_24B: u8 = 0xc0 | (3 << 4);
pub const ZIP_INT_8B:  u8 = 0xfe;



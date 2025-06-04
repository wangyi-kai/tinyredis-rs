pub mod ziplist;
mod lib;
mod error;
mod test;

const ZIPLIST_HEADER_SIZE: u32 = 10;
const ZIPLIST_END_SIZE: u32 = 1;
const ZIP_END: u8 = 255;
const ZIP_BIG_PREVLEN: u8 = 254;
const ZIPLIST_LENGTH_OFFSET: usize = 8;

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

const ZIP_INT_IMM_MIN: u8 = 0xf1;    /* 11110001 */
const ZIP_INT_IMM_MAX: u8 = 0xfd;    /* 11111101 */
const ZIP_INT_IMM_MASK: u8 = 0x0f;

const ZIP_ENCODING_SIZE_INVALID: u8 =  0xff;
const LONG_STR_SIZE: usize = 21;

const INT_24_MAX: i64 = 0x7fffff;
const INT_24_MIN: i64 = -INT_24_MAX - 1;



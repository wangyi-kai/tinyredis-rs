mod test;
pub mod intset;
mod lib;

const INTSET_ENC_INT16: u8 = 2;
const INTSET_ENC_INT32: u8 = 4;
const INTSET_ENC_INT64: u8 = 8;
const INT8_MIN: i64 = -128;
const INT16_MIN: i64 = -32767 - 1;
const INT32_MIN: i64 = -2147483647 - 1;
const INT64_MIN: i64 = 9223372036854775807i64 - 1;
const INT8_MAX: i64 = 128;
const INT16_MAX: i64 = 32767;
const INT32_MAX: i64 = 2147483647;
const INT64_MAX: i64 = 9223372036854775807i64;
const SIZE_MAX: usize = 18446744073709551615;

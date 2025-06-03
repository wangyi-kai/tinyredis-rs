mod quicklist;
mod iter;
mod bookmark;
mod lib;

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
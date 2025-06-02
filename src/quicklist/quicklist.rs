use std::ptr::NonNull;

const QL_FILL_BITS: i32 = 16;
const QL_COMP_BITS: u32 = 16;
const QL_BM_BITS: u32 = 4;

pub struct QuickListNode {
    prev: Option<NonNull<QuickListNode>>,
    next: Option<NonNull<QuickListNode>>,
    entry: String,
    /// entry size in bytes
    sz: usize,
    /// count of items in listpack
    count: u32,
    /// RAW==1 or LZF==2
    encoding: u32,
    /// PLAIN==1 or PACKED==2
    container: u32,
    /// was this node previous compressed?
    recompress: u32,
    /// node can't compress; too small
    attempted_compress: u32,
    /// prevent compression of entry that will be used later
    dont_compress: usize,
    /// more bits to steal for future usage
    extra: u32,
}

pub struct QuickListBookMark {
    node: Option<NonNull<QuickListNode>>,
    name: String,
}

pub struct QuickList {
    head: Option<NonNull<QuickListNode>>,
    tail: Option<NonNull<QuickListNode>>,
    /// total count of all entries in all listpacks
    count: u64,
    /// number of quicklistNodes
    len: u64,
    /// fill factor for individual nodes
    fill: i32,
    /// depth of end nodes not to compress
    compress: u32,
    bookmark_count: u32,
    bookmarks: Vec<QuickListBookMark>,
}


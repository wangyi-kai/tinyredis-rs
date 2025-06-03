use std::ptr::NonNull;
use crate::quicklist::{COMPRESS_MAX, FILL_MAX, MIN_COMPRESS_BYTES, MIN_COMPRESS_IMPROVE, QUICKLIST_NODE_CONTAINER_PACKED, QUICKLIST_NODE_CONTAINER_PLAIN, QUICKLIST_NODE_ENCODING_LZF, QUICKLIST_NODE_ENCODING_RAW};
use crate::quicklist::lib::QuickListLzf;

pub struct QuickListNode {
    prev: Option<NonNull<QuickListNode>>,
    next: Option<NonNull<QuickListNode>>,
    entry: Vec<u8>,
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

impl QuickListNode {
    pub fn create() -> Self {
        Self {
            prev: None,
            next: None,
            entry: Vec::new(),
            sz: 0,
            count: 0,
            encoding: QUICKLIST_NODE_ENCODING_RAW,
            container: QUICKLIST_NODE_CONTAINER_PACKED,
            recompress: 0,
            attempted_compress: 0,
            dont_compress: 0,
            extra: 0,
        }
    }

    pub fn compressed(&mut self) {
        if self.dont_compress != 0 {
            return;
        }
        assert!(self.prev.is_some() && self.next.is_some());
        self.recompress = 0;
        if self.sz < MIN_COMPRESS_BYTES {
            return;
        }

        let compress = match lzf::compress(&self.entry) {
            Ok(lzf) => {
                lzf
            }
            Err(_) => {
                return
            }
        };
        let mut lzf = QuickListLzf::new();
        lzf.set(compress.len(), compress);
        if lzf.sz == 0 || (lzf.sz + MIN_COMPRESS_IMPROVE) >= self.sz {
            return;
        }
        self.entry = lzf.to_u8();
        self.encoding = QUICKLIST_NODE_ENCODING_LZF;
    }

    pub fn get_lzf(&self) -> QuickListLzf {
        let lzf = QuickListLzf::from_u8(&self.entry);
        lzf
    }

    pub fn decompress(&mut self) {
        self.recompress = 0;
        let lzf = QuickListLzf::from_u8(&self.entry);
        let decompress = lzf::decompress(&lzf.compressed, lzf.sz).unwrap();
        let len = decompress.len();
        if len == 0 {
            return;
        }
        self.sz = len;
        self.entry = decompress;
        self.encoding = QUICKLIST_NODE_ENCODING_RAW;
    }
}

pub fn decompress_node(node: Option<NonNull<QuickListNode>>) {
    unsafe {
        if node.is_some() && (*node.unwrap().as_ptr()).encoding == QUICKLIST_NODE_ENCODING_LZF {
            (*node.unwrap().as_ptr()).decompress();
        }
    }
}

pub fn decompress_node_for_use(node: Option<NonNull<QuickListNode>>) {
    unsafe {
        if node.is_some() && (*node.unwrap().as_ptr()).encoding == QUICKLIST_NODE_ENCODING_LZF {
            (*node.unwrap().as_ptr()).decompress();
            (*node.unwrap().as_ptr()).recompress = 1;
        }
    }
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

impl QuickList {
    pub fn create() -> Self {
        Self {
            head: None,
            tail: None,
            count: 0,
            len: 0,
            fill: -2,
            compress: 0,
            bookmark_count: 0,
            bookmarks: vec![],
        }
    }

    pub fn new(fill: i32, compress: i32) -> Self {
        let mut quick_list = QuickList::create();
        quick_list.set_options(fill, compress);
        quick_list
    }

    pub fn set_compress_depth(&mut self, mut compress: i32) {
        if compress > COMPRESS_MAX as i32 {
            compress = COMPRESS_MAX as i32;
        } else if compress < 0 {
            compress = 0;
        }
        self.compress = compress as u32
    }

    pub fn set_fill(&mut self, mut fill: i32) {
        if fill > FILL_MAX {
            fill = FILL_MAX;
        } else if fill < -5 {
            fill = -5;
        }
        self.fill = fill;
    }

    pub fn set_options(&mut self, depth: i32, fill: i32) {
        self.set_fill(fill);
        self.set_compress_depth(depth);
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn release(&mut self) {
        let mut current = self.head;
        let mut len = self.len;
        unsafe {
            while len > 0 {
                len -= 1;
                let next = (*current.unwrap().as_ptr()).next;
                (*current.unwrap().as_ptr()).entry = Vec::new();
                self.count -= (*current.unwrap().as_ptr()).count as u64;
                self.len -= 1;
                current = next;
            }
        }
    }

    pub fn compress(&self, node: QuickListNode) {
        if self.len == 0 {
            return;
        }
        unsafe {
            assert!((*self.head.unwrap().as_ptr()).recompress == 0 && (*self.tail.unwrap().as_ptr()).recompress == 0);
        }
        if self.compress != 0 || self.len < (self.compress * 2) as u64 {
            return;
        }
        let forward = self.head;
        let reverse = self.tail;
        let mut depth = 0;
        while depth < self.compress {

        }
    }
}


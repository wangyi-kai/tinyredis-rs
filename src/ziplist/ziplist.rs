use crate::ziplist::{ZIP_END, ZIPLIST_END_SIZE, ZIPLIST_HEADER_SIZE};
use crate::ziplist::lib::decode_prev_len;

#[derive(Clone, Debug)]
pub struct ZipListEntry {
    s_val: String,
    s_len: u32,
    l_val: i64,
}

pub struct ZlEntry {
    /// length of prev entry length info
    prev_raw_len_size: u32,
    /// prev entry length
    prev_raw_len: u32,
    /// length of cur entry length info
    len_size: u32,
    /// cur entry length
    len: u32,
    /// cur entry head length
    head_size: u32,
    /// cur entry data encode
    encoding: Vec<u8>,
}

fn write_bytes(buf: &mut [u8], value: u32) {
    let bytes = value.to_be_bytes();
    buf.copy_from_slice(&bytes);
}

pub struct ZipList {
    data: Vec<u8>,
}

impl ZipList {
    pub fn new() -> Self {
        let bytes = ZIPLIST_HEADER_SIZE + ZIPLIST_END_SIZE;
        let mut zl = vec![0u8; bytes];
        let head_vec = ZIPLIST_HEADER_SIZE.to_be_bytes();
        zl[0..4].copy_from_slice(&bytes.to_be_bytes());
        zl[4..8].copy_from_slice(&head_vec);
        zl[8..10].copy_from_slice(&vec![0, 0]);
        zl[bytes - 1] = ZIP_END;

        Self { data: zl }
    }

    fn ziplist_len(&self) -> usize {
        let buf = &self.data[0..4];
        u32::from_be_bytes(buf.try_into().unwrap()) as usize
    }

    fn head_offset(&self) -> usize {
        ZIPLIST_HEADER_SIZE
    }

    fn tail_offset(&self) -> usize {
        let buf = &self.data[4..8];
        u32::from_be_bytes(buf.try_into().unwrap()) as usize
    }

    fn last_bytes(&self) -> usize {
        self.ziplist_len() - ZIPLIST_END_SIZE
    }

    pub fn push(&mut self, data: &[u8], is_head: i32) {
        let pos = if is_head == 0 {
            self.head_offset()
        } else {
            self.last_bytes()
        };
        self.insert(pos, data)
    }

    pub fn insert(&mut self, pos: usize, data: &[u8]) {
        let cur_len = u32::from_be_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]) as usize;
        let mut prev_len_size = 0;
        let mut prev_len = 0;

        if self.data[pos] != ZIP_END {
            (prev_len_size, prev_len) = decode_prev_len(&self.data[pos..]);
        } else {
            let tail = self.data[self.tail_offset()..];
            if tail[0] != ZIP_END {

            }
        }
    }
}


use crate::ziplist::{ZIP_ENCODING_SIZE_INVALID, ZIP_END, ZIPLIST_END_SIZE, ZIPLIST_HEADER_SIZE};
use crate::ziplist::error::ZipListError;
use crate::ziplist::lib::{decode_length, decode_prev_len, decode_prev_len_size, encoding_len_size, entry_encoding};

#[derive(Clone, Debug)]
pub struct ZipListEntry {
    s_val: String,
    s_len: u32,
    l_val: i64,
}

pub struct ZlEntry {
    /// length of prev entry length info
    prev_raw_len_size: usize,
    /// prev entry length
    prev_raw_len: usize,
    /// length of cur entry length info
    len_size: usize,
    /// cur entry length
    len: usize,
    /// cur entry head length
    head_size: usize,
    /// cur entry data encode
    encoding: u8,
    buf: Vec<u8>,
}

impl ZlEntry {
    pub fn new(prev_raw_len_size: usize, prev_raw_len: usize, len_size: usize, len: usize, head_size: usize, encoding: u8, data: Vec<u8>) -> Self {
        Self {
            prev_raw_len_size,
            prev_raw_len,
            len_size,
            len,
            head_size,
            encoding,
            buf: data,
        }
    }
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

    pub fn insert(&mut self, pos: usize, data: &[u8]) -> Result<(), ZipListError> {
        let cur_len = u32::from_be_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]) as usize;
        let mut prev_len_size = 0;
        let mut prev_len = 0;

        if self.data[pos] != ZIP_END {
            (prev_len_size, prev_len) = decode_prev_len(&self.data[pos..]);
        } else {
            let tail = self.data[self.tail_offset()..];
            if tail[0] != ZIP_END {
                let prev_len = self.raw_entry_length(cur_len, self.tail_offset())?;
            }
        }
        Ok(())
    }

    fn raw_entry_length(&self, zl_bytes: usize, pos: usize) -> Result<usize, ZipListError> {
        let entry = self.entry_safe(zl_bytes, pos, 0);
        match entry {
            Ok(entry) => {
                Ok(entry.head_size + entry.len)
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    fn entry_safe(
        &self,
        zl_bytes: usize,
        pos: usize,
        validate_len: i32,
    ) -> Result<ZlEntry, ZipListError> {
        fn out_of_range(pos: usize, first: usize, last: usize) -> bool {
            if pos < first || pos > last {
                return true;
            }
            false
        }
        let zl_first = ZIPLIST_HEADER_SIZE;
        let zl_last = zl_bytes - ZIPLIST_END_SIZE;
        if out_of_range(pos, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(0));
        }
        if pos >= ZIPLIST_HEADER_SIZE && pos + 10 < zl_last {
            let (prev_raw_len_size, prev_raw_len) = decode_prev_len(&self.data[pos..]);
            let encoding = entry_encoding(&self.data[pos+prev_raw_len_size..]);
            let (len_size, len) = decode_length(&self.data[pos..], encoding);
            if len_size == 0 {
                panic!("decode len_size error!");
            }
            let head_size = prev_raw_len_size + len_size;
            if out_of_range(pos + head_size + len, zl_first, zl_last) {
                return Err(ZipListError::OutOfRange(1));
            }
            if validate_len != 0 && out_of_range(pos - prev_raw_len, zl_first, zl_last) {
                return Err(ZipListError::OutOfRange(2));
            }
            let entry = ZlEntry::new(prev_raw_len_size, prev_raw_len, len_size, len, head_size, encoding, self.data[pos..].to_vec());
            return Ok(entry)
        }

        if out_of_range(pos, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(3));
        }
        let prev_raw_len_size = decode_prev_len_size(&self.data[pos..]);
        if out_of_range(pos + prev_raw_len_size, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(4));
        }
        let encoding = entry_encoding(&self.data[pos + prev_raw_len_size..]);
        let len_size = encoding_len_size(encoding);
        if len_size == ZIP_ENCODING_SIZE_INVALID as usize {
            return Err(ZipListError::InValidLenSize);
        }
        if out_of_range(pos + prev_raw_len_size + len_size, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(5));
        }
        let (prev_raw_len_size, prev_raw_len) = decode_prev_len(&self.data[pos..]);
        let (len_size, len) = decode_length(&self.data[pos..], encoding);
        let head_size = prev_raw_len_size + len_size;

        if out_of_range(pos + head_size + len, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(6));
        }
        if validate_len != 0 && out_of_range(pos - prev_raw_len, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(7));
        }
        let entry = ZlEntry::new(prev_raw_len_size, prev_raw_len, len_size, len, head_size, encoding, self.data[pos..].to_vec());

        Ok(entry)
    }
}


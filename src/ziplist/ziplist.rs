use crate::ziplist::{ZIP_ENCODING_SIZE_INVALID, ZIP_END, ZIPLIST_END_SIZE, ZIPLIST_HEADER_SIZE, ZIPLIST_LENGTH_OFFSET};
use crate::ziplist::error::ZipListError;
use crate::ziplist::lib::{decode_length, decode_prev_len, decode_prev_len_size, encoding_len_size, entry_encoding, int_size, is_string, prev_len_bytes_diff, save_integer, store_entry_encoding, store_prev_entry_length, store_prev_entry_length_large, try_encoding};

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

    fn resize(&mut self, len: usize) {
        assert!(len < u32::MAX as usize);
        self.data.resize(len, 0);
        self.data[0..4].copy_from_slice(&len.to_be_bytes());
        self.data[len - 1] = ZIP_END;
    }

    fn ziplist_len(&self) -> usize {
        let buf = &self.data[0..4];
        u32::from_le_bytes(buf.try_into().unwrap()) as usize
    }

    fn head_offset(&self) -> usize {
        ZIPLIST_HEADER_SIZE
    }

    fn tail_offset(&self) -> usize {
        let buf = &self.data[4..8];
        u32::from_le_bytes(buf.try_into().unwrap()) as usize
    }

    fn last_bytes(&self) -> usize {
        self.ziplist_len() - ZIPLIST_END_SIZE
    }

    fn entry_num(&self) -> u16 {
        u16::from_le_bytes(self.data[ZIPLIST_LENGTH_OFFSET..ZIPLIST_LENGTH_OFFSET + 2].try_into().unwrap())
    }

    fn incr_length(&mut self, incr: usize) {
        let len = self.entry_num();
        if len < u16::MAX {
            self.data[ZIPLIST_LENGTH_OFFSET..ZIPLIST_LENGTH_OFFSET + 2].copy_from_slice(&*(len as usize + incr).to_le_bytes());
        }
    }

    pub fn push(&mut self, data: &[u8], is_head: i32) {
        let pos = if is_head == 0 {
            self.head_offset()
        } else {
            self.last_bytes()
        };
    }

    pub fn insert(&mut self, mut pos: usize, s: &str) -> Result<(), ZipListError> {
        let cur_len = u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]) as usize;
        let mut prev_len_size = 0;
        let mut prev_len = 0;
        let mut encoding = 0;
        // initialized to avoid warning. Using a value that is easy to see if for some reason we use it uninitialized.
        let mut value = 123456789;

        if self.data[pos] != ZIP_END {
            (prev_len_size, prev_len) = decode_prev_len(&self.data[pos..]);
        } else {
            let tail = &self.data[self.tail_offset()..];
            if tail[0] != ZIP_END {
                prev_len = self.raw_entry_length(cur_len, self.tail_offset())?;
            }
        }
        let mut req_len = if let Some((v, encode)) = try_encoding(s) {
            encoding = encode;
            value = v;
            int_size(encoding)
        } else {
            s.len()
        };
        req_len += store_prev_entry_length_large(None, prev_len);
        req_len += store_entry_encoding(None, encoding, s.len());

        let mut force_large = 0;
        let mut next_diff = if self.data[pos] != ZIP_END {
            prev_len_bytes_diff(&self.data[pos..], req_len)
        } else {
            0
        };
        if next_diff == -4 && req_len < 4 {
            next_diff = 0;
            force_large = 1;
        }

        let new_len = (cur_len as i32 + req_len as i32 + next_diff) as usize;
        self.resize(new_len);
        let mut new_entry_len = 0;

        if self.data[pos] != ZIP_END {
            let dst_start = pos + req_len;
            let src_start = (pos as i32 - next_diff) as usize;
            let src_end = cur_len - 1;
            self.data.copy_within(src_start..src_end, dst_start);
            if force_large {
                store_prev_entry_length_large(Some(&mut self.data[pos + req_len..]), req_len);
            } else {
                store_prev_entry_length(Some(&mut self.data[pos + req_len..]), req_len);
            }
            self.data[4..8].copy_from_slice(&(self.tail_offset() + req_len).to_le_bytes());
            let entry = self.entry_safe(new_len, pos + req_len, 1);
            match entry {
                Ok(tail) => {
                    new_entry_len = tail.len;
                    if self.data[req_len + tail.head_size + tail.len] != ZIP_END {
                        self.data[4..8].copy_from_slice(&(self.tail_offset() as i32 + next_diff).to_le_bytes())
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        } else {
            self.data[4..8].copy_from_slice(&(new_len - new_entry_len - 1).to_le_bytes());
        }

        pos += store_prev_entry_length(Some(&mut self.data[pos..]), prev_len);
        pos += store_entry_encoding(Some(&mut self.data[pos..]), encoding, s.len());
        if is_string(encoding) {
            self.data[pos..].copy_from_slice(s.as_bytes());
        } else {
            save_integer(&mut self.data[pos..], value, encoding);
        }
        self.incr_length(1);

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


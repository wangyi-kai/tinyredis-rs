use std::str::{from_utf8};

use crate::ziplist::{ZIP_ENCODING_SIZE_INVALID, ZIP_END, ZIPLIST_END_SIZE, ZIPLIST_HEADER_SIZE, ZIPLIST_LENGTH_OFFSET};
use crate::ziplist::error::ZipListError;
use crate::ziplist::lib::{*};
use crate::ziplist::lib::Content::{Integer, Char};

#[derive(Clone, Debug)]
pub struct ZipListEntry {
    s_val: String,
    s_len: u32,
    l_val: i64,
}

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub struct ZlEntry {
    /// length of prev entry length info
    pub prev_raw_len_size: u32,
    /// prev entry length
    pub prev_raw_len: u32,
    /// length of cur entry length info
    pub len_size: u32,
    /// cur entry length
    pub len: u32,
    /// cur entry head length
    pub head_size: u32,
    /// cur entry data encode
    pub encoding: u8,
    pub pos: usize,
}

impl ZlEntry {
    pub fn new(prev_raw_len_size: u32, prev_raw_len: u32, len_size: u32, len: u32, head_size: u32, encoding: u8, pos: usize) -> Self {
        Self {
            prev_raw_len_size,
            prev_raw_len,
            len_size,
            len,
            head_size,
            encoding,
            pos,
        }
    }
}

pub struct ZipList {
    pub data: Vec<u8>,
}

impl ZipList {
    pub fn new() -> Self {
        let bytes= ZIPLIST_HEADER_SIZE + ZIPLIST_END_SIZE;
        let mut zl = vec![0u8; bytes as usize];
        zl[0..4].copy_from_slice(&bytes.to_le_bytes());
        zl[4..8].copy_from_slice(&ZIPLIST_HEADER_SIZE.to_le_bytes());
        zl[8..10].copy_from_slice(&vec![0, 0]);
        zl[bytes as usize - 1] = ZIP_END;

        Self { data: zl }
    }

    pub fn create(data: Vec<u8>) -> Self {
        Self {
            data
        }
    }

    pub fn resize(&mut self, len: u32) {
        assert!(len < u32::MAX);
        self.data.resize(len as usize, 0);
        self.data[0..4].copy_from_slice(&len.to_le_bytes());
        self.data[len as usize - 1] = ZIP_END;
    }

    pub fn ziplist_len(&self) -> usize {
        u32::from_le_bytes(self.data[0..4].try_into().unwrap()) as usize
    }

    pub fn head_offset(&self) -> usize {
        ZIPLIST_HEADER_SIZE as usize
    }

    pub fn tail_offset(&self) -> usize {
        u32::from_le_bytes(self.data[4..8].try_into().unwrap()) as usize
    }

    pub fn last_bytes(&self) -> usize {
        self.ziplist_len() - ZIPLIST_END_SIZE as usize
    }

    pub fn entry_num(&mut self) -> u32 {
        let mut len= u16::from_le_bytes(self.data[ZIPLIST_LENGTH_OFFSET..ZIPLIST_LENGTH_OFFSET + 2].try_into().unwrap());
        return if len < u16::MAX {
            len as u32
        } else {
            let mut len: u32 = 0;
            let mut pos = ZIPLIST_HEADER_SIZE as usize;
            let zl_bytes = self.ziplist_len();
            while self.data[pos] != ZIP_END {
                pos += self.raw_entry_length_safe(zl_bytes, pos).unwrap() as usize;
                len += 1;
            }
            if len < u16::MAX as u32{
                self.data[ZIPLIST_LENGTH_OFFSET..ZIPLIST_LENGTH_OFFSET + 2].copy_from_slice(&(len as u16).to_le_bytes());
            }
            len
        }
    }

    fn incr_length(&mut self, incr: i32) {
        let len = self.entry_num();
        if len < u16::MAX as u32 {
            self.data[ZIPLIST_LENGTH_OFFSET..ZIPLIST_LENGTH_OFFSET + 2].copy_from_slice(&((len as i32 + incr) as u16).to_le_bytes());
        }
    }

    pub fn push(&mut self, s: &str, is_head: bool) -> Result<(), ZipListError> {
        let pos = if is_head {
            self.head_offset()
        } else {
            self.last_bytes()
        };
        self.insert(pos, s)
    }

    fn update_tail_offset(&mut self, len: u32) {
        self.data[4..8].copy_from_slice(&len.to_le_bytes())
    }

    pub fn insert(&mut self, mut pos: usize, s: &str) -> Result<(), ZipListError> {
        let cur_len = u32::from_le_bytes(self.data[0..4].try_into().unwrap()) as usize;
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
                prev_len = self.raw_entry_length_safe(cur_len, self.tail_offset())?;
            }
        }
        let mut req_len = if let Some((v, encode)) = try_encoding(s) {
            encoding = encode;
            value = v;
            int_size(encoding)
        } else {
            s.len() as u32
        };
        req_len += store_prev_entry_length(None, prev_len);
        req_len += store_entry_encoding(None, encoding, s.len() as u32);

        let mut force_large = false;
        let mut next_diff = if self.data[pos] != ZIP_END {
            prev_len_bytes_diff(&self.data[pos..], req_len)
        } else {
            0
        };
        if next_diff == -4 && req_len < 4 {
            next_diff = 0;
            force_large = true;
        }

        let new_len = (cur_len as i32 + req_len as i32 + next_diff) as u32;
        self.resize(new_len);
        let mut new_entry_len = 0;

        if self.data[pos] != ZIP_END {
            let dst_start = pos + req_len as usize;
            let src_start = (pos as i32 - next_diff) as usize;
            let src_end = cur_len - 1;
            self.data.copy_within(src_start..src_end, dst_start);

            if force_large {
                store_prev_entry_length_large(Some(&mut self.data[pos + req_len as usize..]), req_len);
            } else {
                store_prev_entry_length(Some(&mut self.data[pos + req_len as usize..]), req_len);
            }

            let tail = self.tail_offset();
            self.data[4..8].copy_from_slice(&(tail as u32 + req_len).to_le_bytes());
            let entry = self.entry_safe(new_len as usize, pos + req_len as usize, 1);
            match entry {
                Ok(tail) => {
                    new_entry_len = tail.len;
                    if self.data[(req_len + tail.head_size + tail.len) as usize] != ZIP_END {
                        let tail = self.tail_offset() as i32;
                        self.data[4..8].copy_from_slice(&(tail + next_diff).to_le_bytes())
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        } else {
            self.data[4..8].copy_from_slice(&(new_len - req_len - 1).to_le_bytes());
        }

        if next_diff != 0 {
            self.cascade_update(pos + req_len as usize);
        }

        pos += store_prev_entry_length(Some(&mut self.data[pos..]), prev_len) as usize;
        pos += store_entry_encoding(Some(&mut self.data[pos..]), encoding, s.len() as u32) as usize;
        if is_string(encoding) {
            self.data[pos..pos + s.len()].copy_from_slice(s.as_bytes());
        } else {
            save_integer(&mut self.data[pos..], value, encoding);
        }
        self.incr_length(1);

        Ok(())
    }

    #[inline]
    fn raw_entry_length_safe(&self, zl_bytes: usize, pos: usize) -> Result<u32, ZipListError> {
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

    #[inline]
    pub fn entry_safe(&self, zl_bytes: usize, pos: usize, validate_len: i32) -> Result<ZlEntry, ZipListError> {
        fn out_of_range(pos: usize, first: usize, last: usize) -> bool {
            if pos < first || pos > last {
                return true;
            }
            false
        }
        let zl_first = ZIPLIST_HEADER_SIZE as usize;
        let zl_last = zl_bytes - ZIPLIST_END_SIZE as usize;
        if out_of_range(pos, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(0));
        }
        if pos >= zl_first && pos + 10 < zl_last {
            let (prev_raw_len_size, prev_raw_len) = decode_prev_len(&self.data[pos..]);
            let encoding = entry_encoding(&self.data[pos + prev_raw_len_size as usize..]);
            let (len_size, len) = decode_length(&self.data[pos + prev_raw_len_size as usize..], encoding);
            if len_size == 0 {
                panic!("decode len_size error!");
            }
            let head_size = prev_raw_len_size + len_size;

            if out_of_range(pos + (head_size + len) as usize, zl_first, zl_last) {
                return Err(ZipListError::OutOfRange(1));
            }
            if validate_len != 0 && out_of_range(pos - prev_raw_len as usize, zl_first, zl_last) {
                return Err(ZipListError::OutOfRange(2));
            }
            let entry = ZlEntry::new(prev_raw_len_size, prev_raw_len, len_size, len, head_size, encoding, pos);
            return Ok(entry)
        }

        if out_of_range(pos, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(3));
        }
        let prev_raw_len_size = decode_prev_len_size(&self.data[pos..]);
        if out_of_range(pos + prev_raw_len_size as usize, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(4));
        }
        let encoding= entry_encoding(&self.data[pos + prev_raw_len_size as usize..]);
        let len_size = encoding_len_size(encoding);
        if len_size == ZIP_ENCODING_SIZE_INVALID as u32 {
            return Err(ZipListError::InValidLenSize);
        }
        if out_of_range(pos + (prev_raw_len_size + len_size) as usize, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(5));
        }
        let (prev_raw_len_size, prev_raw_len) = decode_prev_len(&self.data[pos..]);
        let (len_size, len) = decode_length(&self.data[pos + prev_raw_len_size as usize..], encoding);
        let head_size = prev_raw_len_size + len_size;

        if out_of_range(pos + (head_size + len) as usize, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(6));
        }
        if validate_len != 0 && out_of_range(pos - prev_raw_len as usize, zl_first, zl_last) {
            return Err(ZipListError::OutOfRange(7));
        }
        let entry = ZlEntry::new(prev_raw_len_size, prev_raw_len, len_size, len, head_size, encoding, pos);

        Ok(entry)
    }

    pub fn next_entry_position(&self, mut pos: usize) -> usize {
        let zl_bytes = u32::from_le_bytes(self.data[0..4].try_into().unwrap());

        if self.data[pos] == ZIP_END {
            return pos;
        }
        pos += self.zip_raw_entry_length(pos) as usize;
        if self.data[pos] == ZIP_END {
            return pos;
        }
        let _ = self.entry_safe(zl_bytes as usize, pos, 1);
        pos
    }

    pub fn prev_entry_position(&self, mut pos: usize) -> usize {
        if self.data[pos] == ZIP_END {
            let tail = self.tail_offset();
            if self.data[tail] == ZIP_END {
                return 0;
            } else {
                return tail;
            }
        } else if pos == ZIPLIST_HEADER_SIZE as usize {
            return 0;
        } else {
            let (_, prev_len) = decode_prev_len(&self.data[pos..]);
            assert!(prev_len > 0);
            pos -= prev_len as usize;
            let zl_bytes = u32::from_le_bytes(self.data[0..4].try_into().unwrap());
            let _ = self.entry_safe(zl_bytes as usize, pos, 1);
            return pos;
        }
    }

    pub fn zip_get_entry(&self, pos: usize) -> Option<Content> {
        if self.data[pos] == ZIP_END {
            return None;
        }
        let entry = self.zip_entry(pos);
        if is_string(entry.encoding) {
            let start_pos = pos + entry.head_size as usize;
            let len = entry.len;
            let result = from_utf8(&self.data[start_pos..start_pos + len as usize]).unwrap();
            Some(Char(result.to_string()))
        } else {
            let ret = load_integer(&self.data[pos + entry.head_size as usize..], entry.encoding);
            Some(Integer(ret))
        }
    }

    #[inline]
    fn zip_raw_entry_length(&self, pos: usize) -> u32 {
        let entry = self.zip_entry(pos);
        entry.head_size + entry.len
    }

    #[inline]
    pub fn zip_entry(&self, pos: usize) -> ZlEntry {
        let (prev_raw_len_size, prev_raw_len) = decode_prev_len(&self.data[pos..]);
        let encoding = entry_encoding(&self.data[pos + prev_raw_len_size as usize..]);
        let (len_size, len) = decode_length(&self.data[pos + prev_raw_len_size as usize..], encoding);

        assert_ne!(len_size, 0);
        let header_size = prev_raw_len_size + len_size;
        let entry = ZlEntry::new(prev_raw_len_size, prev_raw_len, len_size, len, header_size, encoding, pos);
        entry
    }

    pub fn cascade_update(&mut self, mut pos: usize) {
        if self.data[pos] == ZIP_END {
            return;
        }

        let mut cur = self.zip_entry(pos);
        let mut prev_len = cur.head_size + cur.len;
        let mut raw_len = 0;
        let first_entry_len = prev_len;
        let mut prev_len_size = store_prev_entry_length(None, prev_len);
        let mut prev_offset = pos;
        let cur_len = self.ziplist_len();
        let delta = 4;
        let mut extra = 0;
        let mut cnt = 0;
        let tail = self.tail_offset();
        pos += prev_len as usize;

        while self.data[pos] != ZIP_END {
            if let Ok(entry) = self.entry_safe(cur_len, pos, 0) {
                cur = entry;
                if cur.prev_raw_len == prev_len {
                    break;
                }
                if cur.prev_raw_len_size >= prev_len_size {
                    if cur.prev_raw_len_size == prev_len_size {
                        store_prev_entry_length(Some(&mut self.data[pos..]), prev_len);
                    } else {
                        store_prev_entry_length_large(Some(&mut self.data[pos..]), prev_len);
                    }
                    break;
                }
            } else {
                return;
            }

            // cur.prev_raw_len means cur is the former head entry.
            assert!(cur.prev_raw_len == 0 || cur.prev_raw_len + delta == prev_len);

            // Update prev entry's info and advance the cursor
            raw_len = cur.head_size + cur.len;
            prev_len = raw_len + delta;
            prev_len_size = store_prev_entry_length(None, prev_len);
            prev_offset = pos;
            pos += raw_len as usize;
            extra += delta;
            cnt += 1;
        }

        if extra == 0 {
            return;
        }
        if tail == prev_offset {
            if (extra - delta) != 0 {
                self.update_tail_offset(self.tail_offset() as u32 + (extra - delta));
            }
        } else {
            self.update_tail_offset(self.tail_offset() as u32 + extra);
        }

        self.resize(cur_len as u32 + extra);
        self.data.copy_within(pos..cur_len - 1, pos + extra as usize);
        pos += extra as usize;

        while cnt > 0 {
            let cur = self.zip_entry(prev_offset);
            raw_len = cur.head_size + cur.len;
            self.data.copy_within(prev_offset + cur.prev_raw_len_size as usize..prev_offset + raw_len as usize, pos - (raw_len - cur.prev_raw_len_size) as usize);
            pos -= (raw_len + delta) as usize;

            if cur.prev_raw_len == 0 {
                store_prev_entry_length(Some(&mut self.data[pos..]), first_entry_len);
            } else {
                store_prev_entry_length(Some(&mut self.data[pos..]), cur.prev_raw_len + delta);
            }
            prev_offset -= cur.prev_raw_len as usize;
            cnt -= 1;
        }
    }

    pub fn delete(&mut self, pos: &mut usize) -> Result<(), ZipListError> {
        let offset = *pos;
        let res = self._delete(offset, 1);
        *pos = offset;
        res
    }

    pub fn _delete(&mut self, mut pos: usize, num: usize) -> Result<(), ZipListError> {
        let mut deleted = 0;

        let mut zl_bytes = self.ziplist_len();
        let first = self.zip_entry(pos);
        let mut next_dif = 0;

        for i in 0..num {
            if self.data[pos] == ZIP_END {
                break;
            }
            pos += self.raw_entry_length_safe(zl_bytes, pos)? as usize;
            deleted += 1;
        }
        assert!(pos >= first.pos);
        let tot_len = pos - first.pos;
        if tot_len > 0 {
            let mut set_tail = 0;
            if self.data[pos] != ZIP_END {
                next_dif = prev_len_bytes_diff(&self.data[pos..], first.prev_raw_len);
                pos = (pos as i32 -  next_dif) as usize;
                assert!(pos >= first.pos && pos < zl_bytes - 1);
                store_prev_entry_length(Some(&mut self.data[pos..]), first.prev_raw_len);
                // update offset for tail
                set_tail = self.tail_offset() - tot_len;

                let tail = self.entry_safe(zl_bytes, pos, 1)?;
                if self.data[(tail.head_size + tail.len) as usize] != ZIP_END {
                    set_tail = (set_tail as i32 + next_dif) as usize;
                }
                let bytes_to_move = zl_bytes - pos - 1;
                self.data.copy_within(pos..pos+bytes_to_move, first.pos);
            } else {
                set_tail = first.pos - first.prev_raw_len as usize;
            }

            zl_bytes -= (tot_len as i32 - next_dif) as usize;
            self.resize(zl_bytes as u32);
            pos = first.pos;
            // Update record count
            self.incr_length(-deleted);
            assert!(set_tail <= zl_bytes - ZIPLIST_END_SIZE as usize);
            self.data[4..8].copy_from_slice(&(set_tail as u32).to_le_bytes());

            if next_dif != 0 {
                self.cascade_update(pos);
            }
        }
        Ok(())
    }

    pub fn zip_index(&self, mut index: i32) -> usize {
        let mut prev_len = 0;
        let bytes = self.ziplist_len();
        let mut pos = 0;

        if index < 0 {
            index = (-index) - 1;
            pos = self.tail_offset();
            if self.data[pos] != ZIP_END {
                let mut prev_len_size = decode_prev_len_size(&self.data[pos..]);
                assert!((pos + prev_len_size as usize) < bytes - 1);
                (prev_len_size, prev_len) = decode_prev_len(&self.data[pos..]);
                while prev_len > 0 && index != 0 {
                    index -= 1;
                    pos -= prev_len as usize;
                    assert!(pos >= ZIPLIST_HEADER_SIZE as usize && pos < (bytes - ZIPLIST_END_SIZE as usize));
                    (prev_len_size, prev_len) = decode_prev_len(&self.data[pos..]);
                }
            }
        } else {
            pos = ZIPLIST_HEADER_SIZE as usize;
            while index > 0 {
                index -= 1;
                if let Ok(raw_len) = self.raw_entry_length_safe(bytes, pos) {
                    pos += raw_len as usize;
                } else {
                    return 0;
                };
                if self.data[pos] == ZIP_END {
                    break;
                }
            }
        }
        if self.data[pos] == ZIP_END || index > 0 {
            return 0;
        }
        match self.entry_safe(bytes, pos, 1) {
            Ok(_) => { }
            Err(_) => {
                return 0;
            }
        }
        pos
    }

    pub fn delete_range(&mut self, index: i32, num: usize) -> Result<(), ZipListError> {
        let pos = self.zip_index(index);
        if pos == 0 {
            return Ok(())
        } else {
            self._delete(pos, num)
        }
    }

    pub fn get(&self, pos: usize, sstr: &mut String, slen: &mut u32, sval: &mut i64) -> bool {
        if pos == 0 || self.data[pos] == ZIP_END {
            return false;
        }
        *sstr = "".to_string();
        *slen = 0;

        let entry = self.zip_entry(pos);
        if is_string(entry.encoding) {
            *slen = entry.len;
            let start = pos + entry.head_size as usize;
            let content = from_utf8(&self.data[start..start + entry.len as usize]).unwrap().to_string();
            *sstr = content;
        } else {
            *sval = load_integer(&self.data[pos + entry.head_size as usize..], entry.encoding);
        }
        true
    }

    pub fn replace(&mut self, mut pos: usize, s: &str) -> Result<(), ZipListError> {
        let entry = self.zip_entry(pos);

        let mut encoding = 0;
        let mut value = 123456789;

        let mut req_len = if let Some((v, e)) = try_encoding(s) {
            value = v;
            encoding = e;
            int_size(e)
        } else {
            s.len() as u32
        };
        req_len += store_entry_encoding(None, encoding, s.len() as u32);

        if req_len == (entry.len_size + entry.len) {
            pos += entry.prev_raw_len_size as usize;
            pos += store_entry_encoding(Some(&mut self.data[pos..]), encoding, s.len() as u32) as usize;
            if is_string(encoding) {
                self.data[pos..pos + s.len()].copy_from_slice(s.as_bytes());
            } else {
                save_integer(&mut self.data[pos..], value, encoding);
            }
        } else {
            self.delete(&mut pos)?;
            self.insert(pos, s)?;
        }

        Ok(())
    }

    pub fn compare(&self, pos: usize, sstr: &str) -> bool {
        if self.data[pos] == ZIP_END {
            return false;
        }

        let entry = self.zip_entry(pos);
        if is_string(entry.encoding) {
            if entry.len == sstr.len() as u32 {
                let s = from_utf8(&self.data[pos + entry.head_size as usize..pos + (entry.head_size + entry.len) as usize]).unwrap();
                return s == sstr;
            } else {
                return false;
            }
        } else if let Some((value, _)) = try_encoding(sstr) {
            let zval = load_integer(&self.data[pos + entry.head_size as usize..], entry.encoding);
            return zval == value;
        }
        false
    }
}


use crate::db::data_structure::intset::lib::intset_value_encoding;
use crate::db::data_structure::intset::*;

use crate::db::data_structure::skiplist::lib::gen_random;

#[derive(Clone, Debug)]
pub struct IntSet {
    pub encoding: u32,
    length: u32,
    pub contents: Vec<u8>,
}

impl IntSet {
    pub fn new() -> Self {
        Self {
            encoding: INTSET_ENC_INT16 as u32,
            length: 0,
            contents: Vec::new(),
        }
    }

    pub fn resize(&mut self, len: u32) {
        let size = (len * self.encoding) as usize;
        assert!(size <= (SIZE_MAX - size_of::<Self>()));
        self.contents.resize(size, 0);
    }

    fn get_encoded(&self, pos: usize, enc: u8) -> i64 {
        if enc == INTSET_ENC_INT64 {
            let offset = pos * 8;
            let bytes: [u8; 8] = self.contents[offset..offset + 8].try_into().unwrap();
            i64::from_le_bytes(bytes)
        } else if enc == INTSET_ENC_INT32 {
            let offset = pos * 4;
            let bytes: [u8; 4] = self.contents[offset..offset + 4].try_into().unwrap();
            i32::from_le_bytes(bytes) as i64
        } else {
            let offset = pos * 2;
            let bytes: [u8; 2] = self.contents[offset..offset + 2].try_into().unwrap();
            i16::from_le_bytes(bytes) as i64
        }
    }

    fn _get(&self, pos: usize) -> i64 {
        let res = self.get_encoded(pos, self.encoding as u8);
        res
    }

    pub fn search(&self, value: i64, pos: Option<&mut usize>) -> bool {
        let mut min = 0;
        let mut max = self.length as i32 - 1;
        let mut mid = 0;
        let mut cur = -1;

        if self.length == 0 {
            if pos.is_some() {
                *pos.unwrap() = 0;
            }
            return false;
        } else {
            if value > self._get(max as usize) {
                if pos.is_some() {
                    *pos.unwrap() = self.length as usize;
                }
                return false;
            } else if value < self._get(0) {
                if pos.is_some() {
                    *pos.unwrap() = 0;
                }
                return false;
            }
        }

        while max >= min {
            mid = (max as u32 + min as u32) >> 1;
            cur = self._get(mid as usize);
            if value > cur {
                min = mid as i32 + 1;
            } else if value < cur {
                max = mid as i32 - 1;
            } else {
                break;
            }
        }

        return if value == cur {
            if pos.is_some() {
                *pos.unwrap() = mid as usize;
            }
            true
        } else {
            if pos.is_some() {
                *pos.unwrap() = min as usize;
            }
            false
        };
    }

    fn inset_set(&mut self, pos: usize, value: i64) {
        let encoding = self.encoding as u8;

        match encoding {
            INTSET_ENC_INT64 => {
                let bytes = value.to_le_bytes();
                let offset = pos * 8;
                self.contents[offset..offset + 8].copy_from_slice(&bytes);
            }
            INTSET_ENC_INT32 => {
                let bytes = (value as i32).to_le_bytes();
                let offset = pos * 4;
                self.contents[offset..offset + 4].copy_from_slice(&bytes);
            }
            INTSET_ENC_INT16 => {
                let bytes = (value as i16).to_le_bytes();
                let offset = pos * 2;
                self.contents[offset..offset + 2].copy_from_slice(&bytes);
            }
            _ => {}
        }
    }

    // Upgrades the intset to a larger encoding and inserts the given integer.
    #[inline(always)]
    pub fn upgrade_and_add(&mut self, value: i64) {
        let cur_encoding = self.encoding as u8;
        let new_encoding = intset_value_encoding(value);
        let mut length = self.length;
        let prepend = if value < 0 { 1 } else { 0 };

        self.encoding = new_encoding as u32;
        self.resize(self.length + 1);

        while length > 0 {
            length -= 1;
            self.inset_set(
                (length + prepend) as usize,
                self.get_encoded(length as usize, cur_encoding),
            );
        }

        if prepend > 0 {
            self.inset_set(0, value);
        } else {
            self.inset_set(self.length as usize, value);
        }
        self.length += 1;
    }

    pub fn move_tail(&mut self, from: usize, to: usize) {
        let mut bytes = self.length as usize - from;
        let encoding = self.encoding as u8;

        let (src, dst) = match encoding {
            INTSET_ENC_INT64 => {
                bytes *= 8;
                (from * 8, to * 8)
            }
            INTSET_ENC_INT32 => {
                bytes *= 4;
                (from * 4, to * 4)
            }
            INTSET_ENC_INT16 => {
                bytes *= 2;
                (from * 2, to * 2)
            }
            _ => (0, 0),
        };

        self.contents.copy_within(src..src + bytes, dst);
    }

    pub fn add(&mut self, value: i64, success: &mut bool) {
        let value_encoding = intset_value_encoding(value);
        let mut pos = 0;
        *success = true;

        if value_encoding > self.encoding as u8 {
            return self.upgrade_and_add(value);
        } else {
            if self.search(value, Some(&mut pos)) {
                *success = false;
                return;
            }
            self.resize(self.length + 1);
            if pos < self.length as usize {
                self.move_tail(pos, pos + 1);
            }
        }

        self.inset_set(pos, value);
        self.length += 1;
    }

    pub fn remove(&mut self, value: i64) {
        let value_encoding = intset_value_encoding(value);
        let mut pos = 0;

        if value_encoding <= self.encoding as u8 && self.search(value, Some(&mut pos)) {
            if pos < (self.length - 1) as usize {
                self.move_tail(pos + 1, pos);
            }
            self.resize(self.length - 1);
            self.length -= 1;
        }
    }

    pub fn find(&self, value: i64) -> bool {
        let value_encoding = intset_value_encoding(value);
        let mut pos = 0;
        return value_encoding <= self.encoding as u8 && self.search(value, Some(&mut pos));
    }

    pub fn intset_random(&self) -> i64 {
        assert!(self.length > 0);
        self._get((gen_random() % self.length) as usize)
    }

    pub fn get_max(&self) -> i64 {
        self._get(self.length as usize - 1)
    }

    pub fn get_min(&self) -> i64 {
        self._get(0)
    }

    pub fn get(&self, pos: usize) -> Option<i64> {
        if pos < self.length as usize {
            return Some(self._get(pos));
        }

        None
    }

    pub fn blob_len(&self) -> usize {
        self.contents.len() + 8
    }

    pub fn get_length(&self) -> u32 {
        self.length
    }

    pub fn validate_integrity(&self, size: usize, deep: i32) -> bool {
        if size < size_of::<Self>() {
            return false;
        }
        let encoding = self.encoding as u8;
        let record_size = match encoding {
            INTSET_ENC_INT64 => 8,
            INTSET_ENC_INT32 => 4,
            INTSET_ENC_INT16 => 2,
            _ => return false,
        };

        let count = self.length as usize;
        if size_of::<Self>() + count * record_size != size {
            return false;
        }

        if count == 0 {
            return false;
        }
        if deep != 0 {
            return true;
        }

        let mut prev = self._get(0);
        for i in 1..count {
            let cur = self._get(i);
            if cur < prev {
                return false;
            }
            prev = cur;
        }
        true
    }
}

use crate::intset::{*};
use crate::intset::lib::intset_value_encoding;

#[derive(Clone, Debug)]
pub struct IntSet {
    encoding: u32,
    length: u32,
    contents: Vec<u8>,
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
        let size = (len * self.encoding) as u64;
        assert!(size <= (SIZE_MAX - size_of::<Self>()) as u64);
        self.contents.resize(size_of::<Self>() + size as usize, 0);
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

    fn get(&self, pos: usize) -> i64 {
        self.get_encoded(pos, self.encoding as u8)
    }

    pub fn search(&self, value: i64, pos: &mut usize) -> u8 {
        let mut min = 0;
        let mut max = self.length - 1;
        let mut mid = 0;
        let mut cur = -1;

        if self.length == 0 {
            *pos = 0;
            return 0;
        } else {
            if value > self.intset_get(max as usize) {
                if *pos != 0 {
                    *pos = self.length as usize;
                }
                return 0;
            } else if value < self.intset_get(0) {
                if *pos != 0 {
                    *pos = self.length as usize;
                }
                return 0;
            }
        }

        while max >= min {
            mid = (max + min) >> 1;
            cur = self.intset_get(mid as usize);
            if value > cur {
                min = mid + 1;
            } else if value < cur {
                max = mid - 1;
            } else {
                break;
            }
        }

        return if value == cur {
            *pos = mid as usize;
            1
        } else {
            *pos = min as usize;
            0
        }
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
                let bytes = value.to_le_bytes();
                let offset = pos * 4;
                self.contents[offset..offset + 4].copy_from_slice(&bytes);
            }
            INTSET_ENC_INT16 => {
                let bytes = value.to_le_bytes();
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
        let length = self.length;
        let prepend = if value < 0 { 1 } else { 0 };

        self.encoding = new_encoding as u32;
        self.resize(self.length + 1);

        while length > 0 {
            self.inset_set((length + prepend) as usize, self.get_encoded(length as usize, cur_encoding));
        }

        if prepend > 0 {
            self.inset_set(0, value);
        } else {
            self.inset_set(self.length as usize, value);
        }
        self.length += 1;
    }

    pub fn move_tail(&mut self, from: u32, to: u32) {
        let mut bytes = self.length - from;
        let encoding = self.encoding;

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
            _ => { (0, 0)}
        };

        self.contents.copy_within(src as usize..(src + bytes) as usize, dst as usize);
    }

    pub fn add(&mut self, value: i64) {
        let value_encoding = intset_value_encoding(value);
        let mut pos = 0;

        if value_encoding > self.encoding as u8 {
            self.upgrade_and_add(value)
        } else {
            if self.search(value, &mut pos) != 0 {
                return;
            }
            self.resize(self.length + 1);
            
        }
    }
}
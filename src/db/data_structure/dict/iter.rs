use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::lib::*;
use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::db::data_structure::dict::iter_mut::DictIterMut;

pub struct DictIter {
    pub dict: *const Dict,
    table: usize,
    index: u64,
    safe: bool,
    remaining: usize,
    entry: Option<NonNull<DictEntry>>,
    //_marker: PhantomData<&'a DictEntry<V>>,
}

impl DictIter {
    pub fn new(dict: &Dict) -> Self {
        let mut iter = DictIter {
            dict,
            table: 0,
            index: 0,
            safe: false,
            entry: None,
            remaining: dict.dict_size() as usize,
            //_marker: PhantomData,
        };
        unsafe {
            iter.advance_next_entry();
        }
        iter
    }

    pub fn new_safe(dict: &Dict) -> Self {
        let mut iter = DictIter {
            dict,
            table: 0,
            index: 0,
            safe: true,
            entry: None,
            remaining: dict.dict_size() as usize,
            //_marker: PhantomData,
        };
        unsafe {
            iter.advance_next_entry();
        }
        iter
    }

    unsafe fn advance_next_entry(&mut self) {
        loop {
            if let Some(entry) = self.entry {
                unsafe {
                    if let Some(next_entry) = (*entry.as_ptr()).next {
                        self.entry = Some(next_entry);
                        return
                    }
                }
                self.entry = None;
            }

            while self.table < 2 {
                let table_size = dict_size((*self.dict).ht_size_exp[self.table]);
                if table_size == 0 {
                    self.table += 1;
                    self.index = 0;
                    continue;
                }
                while self.index < table_size {
                    if let Some(entry) = (*self.dict).ht_table[self.table][self.index as usize] {
                        self.entry = Some(entry);
                        self.index += 1;
                        return;
                    }
                    self.index += 1;
                }
                self.table += 1;
                self.index = 0;
            }
            break;
        }
    }
}

impl Iterator for DictIter {
    type Item = *const DictEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        if let Some(entry) = self.entry {
            unsafe {
                if let Some(next_entry) = (*entry.as_ptr()).next {
                    self.entry = Some(next_entry);
                } else {
                    self.entry = None;
                    self.advance_next_entry();
                }
                self.remaining -= 1;
                return Some(&*entry.as_ptr())
            }
        }
        None
    }
}


impl Dict {
    pub fn iter(&self) -> DictIter {
        DictIter::new(self)
    }

    pub fn safe_iter(&self) -> DictIter {
        DictIter::new_safe(self)
    }

    pub fn iter_mut(&mut self) -> DictIterMut {
        DictIterMut::new(self)
    }

    pub fn safe_iter_mut(&mut self) -> DictIterMut {
        DictIterMut::new_safe(self)
    }
}

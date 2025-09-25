use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::lib::*;
use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::db::data_structure::dict::iter_mut::DictIterMut;

pub struct DictIter<'a, V> {
    pub dict: &'a Dict<V>,
    table: usize,
    index: u64,
    safe: bool,
    remaining: usize,
    entry: Option<NonNull<DictEntry<V>>>,
    _marker: PhantomData<&'a DictEntry<V>>,
}

impl <'a, V> DictIter<'a, V> {
    pub fn new(dict: &'a Dict<V>) -> Self {
        let mut iter = DictIter {
            dict,
            table: 0,
            index: 0,
            safe: false,
            entry: None,
            remaining: dict.dict_size() as usize,
            _marker: PhantomData,
        };
        iter.advance_next_entry();
        iter
    }

    pub fn new_safe(dict: &'a Dict<V>) -> Self {
        let mut iter = DictIter {
            dict,
            table: 0,
            index: 0,
            safe: true,
            entry: None,
            remaining: dict.dict_size() as usize,
            _marker: PhantomData,
        };
        iter.advance_next_entry();
        iter
    }

    fn advance_next_entry(&mut self) {
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
                let table_size = dict_size(self.dict.ht_size_exp[self.table]);
                if table_size == 0 {
                    self.table += 1;
                    self.index = 0;
                    continue;
                }
                while self.index < table_size {
                    if let Some(entry) = self.dict.ht_table[self.table][self.index as usize] {
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

impl <'a, V> Iterator for DictIter<'a, V> {
    type Item = &'a DictEntry<V>;

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


impl<'a, V> Dict<V> {
    pub fn iter(&self) -> DictIter<V> {
        DictIter::new(self)
    }

    pub fn safe_iter(&self) -> DictIter<V> {
        DictIter::new_safe(self)
    }

    pub fn iter_mut(&mut self) -> DictIterMut<V> {
        DictIterMut::new(self)
    }

    pub fn safe_iter_mut(&mut self) -> DictIterMut<V> {
        DictIterMut::new_safe(self)
    }
}

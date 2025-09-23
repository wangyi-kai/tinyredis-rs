use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::iter::DictIter;
use crate::db::data_structure::dict::lib::dict_size;

pub struct DictIterMut<'a, V> {
    dict: &'a mut Dict<V>,
    table: usize,
    bucket: usize,
    current_entry: Option<NonNull<DictEntry<V>>>,
    remaining: usize,
    _marker: PhantomData<&'a mut DictEntry<V>>,
}

impl <'a, V> DictIterMut<'a, V> {
    pub fn new(dict: &'a mut Dict<V>) -> Self {
        let remaining = dict.dict_size() as usize;
        let mut iter = DictIterMut {
            dict,
            table: 0,
            bucket: 0,
            current_entry: None,
            remaining,
            _marker: PhantomData,
        };
        iter.advance_to_next_entry();
        iter
    }

    fn advance_to_next_entry(&mut self) {
        loop {
            if let Some(entry) = self.current_entry {
                unsafe {
                    if let Some(next_entry) = (*entry.as_ptr()).next {
                        self.current_entry = Some(next_entry);
                        return;
                    }
                }
                self.current_entry = None;
            }

            while self.table < 2 {
                let table_size = dict_size(self.dict.ht_size_exp[self.table]);
                if table_size == 0 {
                    self.table += 1;
                    self.bucket = 0;
                    continue;
                }
                while self.bucket < table_size as usize {
                    if let Some(entry) = self.dict.ht_table[self.table][self.bucket] {
                        self.current_entry = Some(entry);
                        self.bucket += 1;
                        return;
                    }
                    self.bucket += 1;
                }
                self.table += 1;
                self.bucket = 0;
            }
            break;
        }
    }
}

impl <'a, V> Iterator for DictIterMut<'a, V> {
    type Item = &'a mut DictEntry<V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        if let Some(mut entry) = self.current_entry {
            unsafe {
                if let Some(next) = (*entry.as_ptr()).next {
                    self.current_entry = Some(next);
                } else {
                    self.current_entry = None;
                    self.advance_to_next_entry();
                }
                self.remaining -= 1;
                return Some(&mut *entry.as_mut())
            }
        }
        None
    }
}
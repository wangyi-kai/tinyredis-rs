use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::lib::*;
use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::db::data_structure::dict::iter_mut::DictIterMut;

#[derive(Debug)]
pub struct EntryIter<'a, V> {
    cur: Option<&'a DictEntry<V>>,
    _boo: PhantomData<&'a V>,
}

#[derive(Debug)]
pub struct EntryIterMut<'a, V> {
    cur: Option<&'a mut DictEntry<V>>,
    _boo: PhantomData<&'a V>,
}

impl<V> DictEntry<V> {
    pub fn iter(&self) -> EntryIter<V> {
        EntryIter {
            cur: Some(self),
            _boo: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> EntryIterMut<V> {
        EntryIterMut {
            cur: Some(self),
            _boo: PhantomData,
        }
    }
}

impl<'a, V> Iterator for EntryIter<'a, V> {
    type Item = &'a DictEntry<V>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cur) = self.cur {
            unsafe {
                let next = cur.next;
                if next.is_some() {
                    self.cur = Some(&(*next.unwrap().as_ptr()));
                } else {
                    self.cur = None;
                };
                Some(cur)
            }
        } else {
            None
        }
    }
}

impl<'a, V> Iterator for EntryIterMut<'a, V> {
    type Item = &'a mut DictEntry<V>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cur) = self.cur.take() {
            unsafe {
                let next = cur.next;
                if next.is_some() {
                    self.cur = Some(&mut (*next.unwrap().as_ptr()));
                } else {
                    self.cur = None;
                };
                Some(cur)
            }
        } else {
            None
        }
    }
}


pub struct DictIterator<'a, V> {
    pub(crate) dict: Option<&'a mut Dict<V>>,
    pub(crate) table: usize,
    pub(crate) index: i64,
    pub(crate) safe: i64,
    pub(crate) entry: Option<EntryIter<'a, V>>,
}

pub struct DictIter<'a, V> {
    pub dict: &'a Dict<V>,
    table: usize,
    index: usize,
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
                while self.index < table_size as usize {
                    if let Some(entry) = self.dict.ht_table[self.table][self.index] {
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

impl<'a, V> DictIterator<'a, V> {
    pub fn reset(&mut self) {
        if !(self.index == -1 && self.table == 0) {
            if self.safe != 0 {
                self.dict.as_mut().unwrap().resume_rehash()
            }
        }
    }
}

impl<'a, V> Dict<V> {
    pub fn iter(&self) -> DictIter<V> {
        DictIter::new(self)
    }

    pub fn iter_mut(&mut self) -> DictIterMut<V> {
        DictIterMut::new(self)
    }

    pub fn safe_iter(&mut self) -> DictIterator<V> {
        DictIterator {
            dict: Some(self),
            table: 0,
            index: -1,
            safe: 1,
            entry: None,
        }
    }
}

impl<'a, V> Iterator for DictIterator<'a, V> {
    type Item = &'a DictEntry<V>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            loop {
                if let Some(ref mut iter) = self.entry {
                    if let Some(next) = iter.next() {
                        if *next != DictEntry::default() {
                            return Some(next);
                        }
                    }
                    self.entry = None;
                } else {
                    if self.table == 0 && self.index == -1 {
                        if self.safe == 1 {
                            self.dict.as_mut().unwrap().pause_rehash();
                        }
                        if self.dict.as_ref().unwrap().dict_is_rehashing() {
                            self.index = self.dict.as_ref().unwrap().get_rehash_idx() - 1;
                        }
                    }
                }
                self.index += 1;
                if self.index
                    >= (dict_size(self.dict.as_ref().unwrap().ht_size_exp[self.table]) as i64)
                {
                    if self.dict.as_ref().unwrap().dict_is_rehashing() && self.table == 0 {
                        self.table += 1;
                        self.index = 0;
                    } else {
                        break;
                    }
                }
                let entry_iter = (*self.dict.as_ref().unwrap().ht_table[self.table]
                    [self.index as usize]
                    .unwrap()
                    .as_ptr())
                    .iter();
                self.entry = Some(entry_iter);
            }
        }
        None
    }
}

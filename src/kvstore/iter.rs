use crate::data_structure::dict::dict::{Dict, DictEntry};
use crate::data_structure::dict::iter::DictIterator;
use crate::kvstore::kvstore::KvStore;
use std::hash::Hash;
use std::ptr::NonNull;

pub struct KvStoreIterator<'a, V> {
    pub(crate) kvs: *mut KvStore<V>,
    pub(crate) didx: i32,
    pub(crate) next_didx: i32,
    pub(crate) di: Option<DictIterator<'a, V>>,
}

impl<'a, V> KvStoreIterator<'a, V> {
    pub fn next_dict(&mut self) -> Option<NonNull<Dict<V>>> {
        if self.next_didx == -1 {
            return None;
        }
        unsafe {
            if self.didx != -1 && (*self.kvs).get_dict(self.didx as usize).is_some() {
                let iter = self.di.as_mut().unwrap();
                (*iter).reset();
                (*self.kvs).free_dict_if_needed(self.didx as usize);
            }
            self.didx = self.next_didx;
            self.next_didx = (*self.kvs).get_next_non_empty_dict_index(self.didx as usize);
            (*self.kvs).dicts[self.didx as usize]
        }
    }

    pub fn get_current_dict_index(&self) -> i32 {
        unsafe {
            assert!(self.didx >= 0 && self.didx < (*self.kvs).num_dicts as i32);
            self.didx
        }
    }

    pub fn release(&mut self) {
        let iter = self.di.as_mut();
        iter.unwrap().reset();
        unsafe {
            (*self.kvs).free_dict_if_needed(self.didx as usize);
        }
    }
}

impl<'a, V> Iterator for KvStoreIterator<'a, V> {
    type Item = &'a DictEntry<V>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut de = if self.di.as_ref().unwrap().dict.is_some() {
                self.di.as_mut().unwrap().next()
            } else {
                None
            };
            if de.is_none() {
                let d = self.next_dict();
                if d.is_none() {
                    return None;
                }
                let dict_iter = DictIterator {
                    dict: Some(&mut (*d.unwrap().as_ptr())),
                    table: 0,
                    index: -1,
                    safe: 1,
                    entry: None,
                };
                self.di = Some(dict_iter);
                de = self.di.as_mut().unwrap().next();
            }
            de
        }
    }
}

impl<'a, V> Drop for KvStoreIterator<'a, V> {
    fn drop(&mut self) {}
}

pub struct KvStoreDictIterator<'a, V> {
    pub(crate) kvs: *mut KvStore<V>,
    pub(crate) didx: i32,
    pub(crate) di: Option<DictIterator<'a, V>>,
}

impl<'a, V> KvStoreDictIterator<'a, V> {
    pub fn release_dict_iterator(&mut self) {
        unsafe {
            if (*self.kvs).get_dict(self.didx as usize).is_some() {
                self.di.as_mut().unwrap().reset();
                (*self.kvs).free_dict_if_needed(self.didx as usize);
            }
        }
    }
}

impl<'a, V> Iterator for KvStoreDictIterator<'a, V> {
    type Item = &'a DictEntry<V>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let d = (*self.kvs).get_dict(self.didx as usize);
            if d.is_none() {
                return None;
            }
            self.di.as_mut().unwrap().next()
        }
    }
}

impl<'a, V> Drop for KvStoreDictIterator<'a, V> {
    fn drop(&mut self) {}
}

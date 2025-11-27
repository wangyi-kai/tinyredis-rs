use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::iter::{DictIter};
use crate::db::data_structure::dict::iter_mut::DictIterMut;
use crate::db::kvstore::kvstore::KvStore;

use std::ptr::NonNull;

pub struct KvStoreIterator<V> {
    pub(crate) kvs: *mut KvStore<V>,
    pub(crate) didx: i32,
    pub(crate) next_didx: i32,
    pub(crate) di: DictIterMut<V>,
}

impl<V> KvStoreIterator<V> {
    pub fn next_dict(&mut self) -> Option<NonNull<Dict<V>>> {
        if self.next_didx == -1 {
            return None;
        }
        unsafe {
            if self.didx != -1 && (*self.kvs).get_dict(self.didx as usize).is_some() {
                self.di.reset();
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
        self.di.reset();
        unsafe {
            (*self.kvs).free_dict_if_needed(self.didx as usize);
        }
    }
}

impl<V> Iterator for KvStoreIterator<V> {
    type Item = *mut DictEntry<V>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if let Some(entry) = self.di.next() {
                return Some(entry)
            } else {
                if let Some(d) = self.next_dict() {
                    let iter = DictIterMut::new(&mut *d.as_ptr());
                    self.di = iter;
                    return self.di.next()
                } else {
                    return None;
                }
            }
        }
    }
}

impl<V> Drop for KvStoreIterator<V> {
    fn drop(&mut self) {}
}

pub struct KvStoreDictIterator<V> {
    pub(crate) kvs: *mut KvStore<V>,
    pub(crate) didx: i32,
    pub(crate) di: DictIterMut<V>,
}

impl<V> KvStoreDictIterator<V> {
    pub fn _release_dict_iterator(&mut self) {
        unsafe {
            if (*self.kvs).get_dict(self.didx as usize).is_some() {
                self.di.reset();
                (*self.kvs).free_dict_if_needed(self.didx as usize);
            }
        }
    }
}

impl<V> Iterator for KvStoreDictIterator<V> {
    type Item = *mut DictEntry<V>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let d = (*self.kvs).get_dict(self.didx as usize);
            if d.is_none() {
                return None;
            }
            self.di.next()
        }
    }
}

impl<V> Drop for KvStoreDictIterator<V> {
    fn drop(&mut self) {}
}

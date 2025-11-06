use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::iter::{DictIter};
use crate::db::data_structure::dict::iter_mut::DictIterMut;
use crate::db::kvstore::kvstore::KvStore;

use std::ptr::NonNull;

pub struct KvStoreIterator<'a, V> {
    pub(crate) kvs: *mut KvStore<V>,
    pub(crate) didx: i32,
    pub(crate) next_didx: i32,
    pub(crate) di: DictIterMut<'a, V>,
}

impl<'a, V> KvStoreIterator<'a, V> {
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
            (&(*self.kvs).dicts)[self.didx as usize].as_ref().map(|node| *node)
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

impl<'a, V> Iterator for KvStoreIterator<'a, V> {
    type Item = &'a mut DictEntry<V>;
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

impl<'a, V> Drop for KvStoreIterator<'a, V> {
    fn drop(&mut self) {}
}

pub struct KvStoreDictIterator<'a, V> {
    pub(crate) kvs: *mut KvStore<V>,
    pub(crate) didx: i32,
    pub(crate) di: DictIterMut<'a, V>,
}

impl<'a, V> KvStoreDictIterator<'a, V> {
    pub fn _release_dict_iterator(&mut self) {
        unsafe {
            if (*self.kvs).get_dict(self.didx as usize).is_some() {
                self.di.reset();
                (*self.kvs).free_dict_if_needed(self.didx as usize);
            }
        }
    }
}

impl<'a, V> Iterator for KvStoreDictIterator<'a, V> {
    type Item = &'a mut DictEntry<V>;
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

impl<'a, V> Drop for KvStoreDictIterator<'a, V> {
    fn drop(&mut self) {}
}

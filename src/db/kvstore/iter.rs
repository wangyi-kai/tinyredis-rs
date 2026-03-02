use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::iter_mut::DictIterMut;
use crate::db::kvstore::kvstore::KvStore;

use std::ptr::NonNull;

pub struct KvStoreIterator {
    pub(crate) kvs: *mut KvStore,
    pub(crate) didx: i32,
    pub(crate) next_didx: i32,
    pub(crate) di: DictIterMut,
}

unsafe impl Send for KvStoreIterator {}

unsafe impl Sync for KvStoreIterator {}

impl KvStoreIterator {
    pub fn next_dict(&mut self) -> Option<NonNull<Dict>> {
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

impl Iterator for KvStoreIterator {
    type Item = *mut DictEntry;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if let Some(entry) = self.di.next() {
                Some(entry)
            } else {
                if let Some(d) = self.next_dict() {
                    let iter = DictIterMut::new(&mut *d.as_ptr());
                    self.di = iter;
                    self.di.next()
                } else {
                    None
                }
            }
        }
    }
}

impl Drop for KvStoreIterator {
    fn drop(&mut self) {}
}

pub struct KvStoreDictIterator {
    pub(crate) kvs: *mut KvStore,
    pub(crate) didx: i32,
    pub(crate) di: DictIterMut,
}

impl KvStoreDictIterator {
    pub fn _release_dict_iterator(&mut self) {
        unsafe {
            if (*self.kvs).get_dict(self.didx as usize).is_some() {
                self.di.reset();
                (*self.kvs).free_dict_if_needed(self.didx as usize);
            }
        }
    }
}

impl Iterator for KvStoreDictIterator {
    type Item = *mut DictEntry;
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

impl Drop for KvStoreDictIterator {
    fn drop(&mut self) {}
}

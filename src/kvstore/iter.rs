use std::hash::Hash;
use std::ptr::NonNull;
use crate::dict::dict::{Dict, DictEntry};
use crate::kvstore::kvstore::KvStore;
use crate::dict::iter::DictIterator;

pub struct KvStoreIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub(crate) kvs: *mut KvStore<K, V>,
    pub(crate) didx: i32,
    pub(crate) next_didx: i32,
    pub(crate) di: Option<*mut DictIterator<'a, K, V>>,
}

impl <'a, K, V> KvStoreIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn next_dict(&mut self) -> Option<NonNull<Dict<K, V>>> {
        if self.next_didx == -1 {
            return None;
        }
        unsafe {
            if self.didx != -1 && (*self.kvs).get_dict(self.didx as usize).is_some() {
                let mut iter = self.di.unwrap();
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
            assert!(self.didx > 0 && self.didx < (*self.kvs).num_dicts as i32);
            self.didx
        }
    }
}

impl <'a, K, V> Iterator for KvStoreIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    type Item = &'a DictEntry<K, V>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut de = if self.di.is_some() {
                (*self.di.unwrap()).next()
            } else {
                None
            };
            if de.is_none() {
                let d = self.next_dict();
                if d.is_none() {
                    return None;
                }
                de = (*self.di.unwrap()).next();
            }
            de
        }
    }
}

pub struct KvStoreDictIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub(crate) kvs: *mut KvStore<K, V>,
    pub(crate) didx: i32,
    pub(crate) di: Option<*mut DictIterator<'a, K, V>>,
}

impl <'a, K, V> KvStoreDictIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn release_dict_iterator(&mut self) {
        unsafe {
            if (*self.kvs).get_dict(self.didx as usize).is_some() {
                (*self.di.unwrap()).reset();
                (*self.kvs).free_dict_if_needed(self.didx as usize);
            }
        }
    }
}

impl <'a, K, V> Iterator for KvStoreDictIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    type Item = &'a DictEntry<K, V>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let d = (*self.kvs).get_dict(self.didx as usize);
            if d.is_none() {
                return None;
            }
            (*self.di.unwrap()).next()
        }
    }
}



use std::hash::Hash;
use std::ptr::NonNull;
use crate::adlist::adlist::List;
use crate::dict::dict::{Dict, DictEntry};
use crate::dict::lib::{dict_size, DictType};
use crate::kvstore::{KVSTORE_ALLOC_META_KEYS_HIST, KVSTORE_ALLOCATE_DICTS_ON_DEMAND, KVSTORE_FREE_EMPTY_DICTS};
use crate::kvstore::meta::KvStoreDictMetaBase;

#[derive(Clone)]
pub struct KvStoreMetadata {
    key_size_hits: Vec<Vec<u64>>,
}

#[derive(Clone)]
pub struct KvStoreDictMetadata {
    key_size_hits: Vec<Vec<u64>>
}

pub struct KvStore<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    flag: i32,
    pub(crate) dtype: &'a DictType<K, V>,
    pub(crate) dicts: Vec<Option<NonNull<Dict<'a, K, V>>>>,
    num_dicts: u64,
    num_dicts_bits: u64,
    /// List of dictionaries in this kvstore that are currently rehashing
    pub rehashing: List<NonNull<Dict<'a, K, V>>>,
    /// Cron job uses this cursor to gradually resize dictionaries (only used if num_dicts > 1)
    resize_cursor: i32,
    /// The number of allocated dicts.
    allocated_dicts: i32,
    /// The number of non-empty dicts.
    non_empty_dicts: i32,
    /// Total number of keys in this kvstore
    key_count: u64,
    /// Total number of buckets in this kvstore across dictionaries
    bucket_count: u64,
    /// Binary indexed tree (BIT) that describes cumulative key frequencies up until given dict-index
    dict_size_index: vec![],
    /// The overhead of all dictionaries
    overhead_hashtable_lut: usize,
    /// The overhead of dictionaries rehashing
    overhead_hashtable_rehashing: usize,
    /// conditionally allocated based on "flags"
    metadata: Vec<KvStoreMetadata>
}

impl<'a, K, V> KvStore<'a, K, V>
where K: Default + Clone + Eq + Hash + std::fmt::Display,
      V: Default + PartialEq + Clone
{
    // num_dicts_bits is the log2 of the amount of dictionaries needed
    // (e.g. 0 for 1 dict, 3 for 8 dicts)
    pub fn create(dict_type: &'a DictType<K, V>, num_dicts_bits: u64, flag: i32) -> Self {
        unsafe {
            assert!(num_dicts_bits <= 16);
            let mut kv_size = size_of::<Self>();
            if flag & KVSTORE_ALLOC_META_KEYS_HIST != 0 {
                kv_size += size_of::<KvStoreMetadata>();
            }
            let num_dicts = 1 << num_dicts_bits;
            let mut dicts = Vec::new();
            let mut allocated_dicts = 0;
            if flag & KVSTORE_ALLOCATE_DICTS_ON_DEMAND != 0 {
                for i in 0..num_dicts {
                    let d = Dict::create(dict_type);
                    dicts.push(Some(NonNull::new_unchecked(Box::into_raw(Box::new(d)))));
                    allocated_dicts += 1;
                }
            }
            let rehashing = List::create();
            let dict_size_index = if num_dicts > 1 {
                vec![0; 8 * (1 + num_dicts)];
            } else {
                vec![];
            };

            Self {
                flag,
                dtype: dict_type,
                dicts,
                num_dicts: num_dicts as u64,
                num_dicts_bits,
                rehashing,
                resize_cursor: 0,
                allocated_dicts,
                non_empty_dicts: 0,
                key_count: 0,
                bucket_count: 0,
                dict_size_index,
                overhead_hashtable_lut: 0,
                overhead_hashtable_rehashing: 0,
                metadata: Vec::new(),
            }
        }
    }

    pub fn empty(&mut self, call_back: Option<fn(&mut Dict<K, V>)>) {
        unsafe {
            for didx in 0..self.num_dicts as usize {
                let d = self.get_dict(didx);
                if d.is_none() {
                    continue;
                }
                (*d.unwrap().as_ptr()).empty(call_back);
                self.free_dict_if_needed(didx);
            }

            self.rehashing.empty();
            self.key_count = 0;
            self.non_empty_dicts = 0;
            self.resize_cursor = 0;
            self.bucket_count = 0;
            self.overhead_hashtable_rehashing = 0;
            self.overhead_hashtable_lut = 0;
            if !self.dict_size_index.is_empty() {
                self.dict_size_index = Vec::new();
            }
        }
    }

    pub fn release(&mut self) {
        unsafe {
            for didx in 0..self.num_dicts as usize {
                let d = self.get_dict(didx);
                if d.is_none() {
                    continue;
                }
                (*d.unwrap().as_ptr()).release();
            }
            self.dicts = vec![];
            self.rehashing.empty();
            self.dict_size_index = vec![];
        }
    }

    pub fn get_dict(&self, didx: usize) -> Option<NonNull<Dict<K, V>>> {
        self.dicts[didx]
    }

    pub fn dict_is_rehashing_paused(&self, didx: usize) -> bool {
        unsafe {
            let dict = self.dicts[didx].unwrap();
            (*dict.as_ptr()).is_rehash_pause()
        }
    }

    pub fn kvstore_size(&self) -> u64 {
        unsafe {
            if self.num_dicts != 1 {
                self.key_count
            } else {
                let d = self.dicts[0];
                if d.is_some() {
                    (*d.unwrap().as_ptr()).dict_size() as u64
                } else {
                    0
                }
            }
        }
    }

    pub fn kvstore_buckets(&self) -> u64 {
        unsafe {
            if self.num_dicts != 1 {
                self.bucket_count
            } else {
                let d = self.dicts[0];
                if d.is_some() {
                    (*d.unwrap().as_ptr()).dict_buckets()
                } else {
                    0
                }
            }
        }
    }

    fn kvstore_dict_size(&self, didx: usize) -> usize {
        unsafe {
            let d = self.dicts[didx];
            (*d.unwrap().as_ptr()).dict_size() as usize
        }
    }
    fn kvstore_dict_is_rehashing_paused(&self, didx: usize) -> bool {
        unsafe {
            let d = self.dicts[didx];
            return if d.is_some() {
                (*d.unwrap().as_ptr()).dict_is_rehash_paused()
            } else {
                false
            }
        }
    }

    fn free_dict_if_needed(&mut self, didx: usize) {
        unsafe {
            if self.flag & KVSTORE_FREE_EMPTY_DICTS == 0 || self.get_dict(didx).is_none() || self.kvstore_dict_size(didx) != 0 || self.kvstore_dict_is_rehashing_paused(didx) {
                return;
            }
            (*self.dicts[didx].unwrap().as_ptr()).release();
            self.dicts[didx] = None;
            self.allocated_dicts -= 1;
        }
    }


}
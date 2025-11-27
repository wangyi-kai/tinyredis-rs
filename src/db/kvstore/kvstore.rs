use std::mem::size_of;
use crate::db::data_structure::adlist::adlist::{LinkList, Node};
use crate::db::data_structure::dict::dict::{Dict, DictEntry};
use crate::db::data_structure::dict::lib::{entry_mem_usage, DictScanFunction};
use crate::db::kvstore::iter::{KvStoreDictIterator, KvStoreIterator};
use crate::db::kvstore::lib::{KvStoreExpandShouldSkipDictIndex, KvStoreScanShouldSkipDict};
use crate::db::kvstore::{
    KVSTORE_ALLOCATE_DICTS_ON_DEMAND, KVSTORE_ALLOC_META_KEYS_HIST, KVSTORE_FREE_EMPTY_DICTS,
};
use rand::Rng;
use std::ptr::NonNull;
use std::time::Instant;
use crate::db::data_structure::dict::iter_mut::DictIterMut;
use crate::db::object::RedisObject;

#[derive(Clone)]
pub struct KvStoreMetadata {
    key_size_hits: Vec<Vec<u64>>,
}

#[derive(Clone)]
pub struct KvStoreDictMetadata {
    key_size_hits: Vec<Vec<u64>>,
}

unsafe impl<V: Send> Send for KvStore<V> {}
unsafe impl<V: Sync> Sync for KvStore<V> {}

pub struct KvStore<V> {
    flag: i32,
    pub(crate) dicts: Vec<Option<NonNull<Dict<V>>>>,
    pub(crate) num_dicts: u64,
    num_dicts_bits: u64,
    /// List of dictionaries in this kvstore that are currently rehashing
    pub rehashing: LinkList<NonNull<Dict<V>>>,
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
    dict_size_index: Vec<u64>,
    /// The overhead of all dictionaries
    overhead_hashtable_lut: usize,
    /// The overhead of dictionaries rehashing
    overhead_hashtable_rehashing: usize,
    // conditionally allocated based on "flags"
    // metadata: Vec<Box<dyn Any>>,
}

impl<V> KvStore<V> {
    // num_dicts_bits is the log2 of the amount of dictionaries needed
    // (e.g. 0 for 1 dict, 3 for 8 dicts)
    pub fn create(num_dicts_bits: u64, flag: i32) -> Self {
        unsafe {
            assert!(num_dicts_bits <= 16);
            let mut kv_size = size_of::<Self>();
            if flag & KVSTORE_ALLOC_META_KEYS_HIST != 0 {
                kv_size += size_of::<KvStoreMetadata>();
            }
            let num_dicts = 1 << num_dicts_bits;
            let mut dicts = vec![None; num_dicts];
            let mut allocated_dicts = 0;
            if (flag & KVSTORE_ALLOCATE_DICTS_ON_DEMAND) == 0 {
                for i in 0..num_dicts {
                    let d = Dict::create();
                    dicts[i] = Some(NonNull::new_unchecked(Box::into_raw(Box::new(d))));
                    allocated_dicts += 1;
                }
            }
            let rehashing = LinkList::create();
            let dict_size_index = if num_dicts > 1 {
                vec![0; 8 * (1 + num_dicts)].to_vec()
            } else {
                Vec::new()
            };

            Self {
                flag,
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
            }
        }
    }

    pub fn create_dict_if_needed(&mut self, didx: i32) -> Option<NonNull<Dict<V>>> {
        let d = self.dicts[didx as usize];
        if d.is_some() {
            return d;
        }
        unsafe {
            let dict = Dict::create();
            self.dicts[didx as usize] = Some(NonNull::new_unchecked(Box::into_raw(Box::new(dict))));
            self.allocated_dicts += 1;
            self.dicts[didx as usize]
        }
    }

    pub fn empty(&mut self, call_back: Option<fn(&mut Dict<V>)>) {
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

    pub fn get_dict(&self, didx: usize) -> Option<NonNull<Dict<V>>> {
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
            if d.is_some() {
                (*d.unwrap().as_ptr()).dict_is_rehash_paused()
            } else {
                false
            }
        }
    }

    pub fn free_dict_if_needed(&mut self, didx: usize) {
        unsafe {
            if self.flag & KVSTORE_FREE_EMPTY_DICTS == 0
                || self.get_dict(didx).is_none()
                || self.kvstore_dict_size(didx) != 0
                || self.kvstore_dict_is_rehashing_paused(didx)
            {
                return;
            }
            (*self.dicts[didx].unwrap().as_ptr()).release();
            self.dicts[didx] = None;
            self.allocated_dicts -= 1;
        }
    }

    pub fn mem_usge(&self) -> usize {
        let mut mem = size_of::<Self>();
        let keys_count = self.kvstore_size();
        mem += keys_count as usize * entry_mem_usage::<V>();
        mem += self.kvstore_buckets() as usize * size_of::<&DictEntry<V>>();
        mem += self.allocated_dicts as usize * size_of::<Dict<V>>();
        mem += self.rehashing.length() * size_of::<Node<Dict<V>>>();
        if !self.dict_size_index.is_empty() {
            mem += (self.num_dicts + 1) as usize * size_of::<u64>();
        }

        mem
    }

    pub fn get_and_clear_dict_index_from_cursor(&self, cursor: &mut u64) -> i32 {
        if self.num_dicts == 1 {
            return 0;
        }
        let didx = *cursor & (self.num_dicts - 1);
        *cursor >>= self.num_dicts_bits;
        didx as i32
    }

    pub fn kvstore_scan(
        &mut self,
        mut cursor: u64,
        only_didx: i32,
        scan_cb: Option<DictScanFunction<V>>,
        skip_cb: Option<KvStoreScanShouldSkipDict<V>>,
    ) -> u64 {
        unsafe {
            let mut _cursor = 0;
            let mut didx = self.get_and_clear_dict_index_from_cursor(&mut cursor);
            if only_didx > 0 {
                if didx < only_didx {
                    assert!(only_didx < self.num_dicts as i32);
                    didx = only_didx;
                    cursor = 0;
                } else if didx > only_didx {
                    return 0;
                }
            }
            let d = self.get_dict(didx as usize);
            let skip = if !d.is_none() || (scan_cb.is_some() && skip_cb.is_some()) {
                true
            } else {
                false
            };
            if !skip {
                _cursor = (*d.unwrap().as_ptr()).scan(cursor, scan_cb.unwrap());
                self.free_dict_if_needed(didx as usize);
            }
            if _cursor == 0 || skip {
                if only_didx >= 0 {
                    return 0;
                }
                didx = self.get_next_non_empty_dict_index(didx as usize);
            }
            if didx == -1 {
                return 0;
            }
            self.add_dict_index_to_cursor(didx, &mut _cursor);
            _cursor
        }
    }

    pub fn expand(
        &mut self,
        new_size: u64,
        skip_cb: Option<KvStoreExpandShouldSkipDictIndex>,
    ) -> bool {
        unsafe {
            for i in 0..self.num_dicts {
                let d = self.get_dict(i as usize);
                if d.is_none() || (skip_cb.is_none() && skip_cb.unwrap()(i as usize) != 0) {
                    continue;
                }
                match (*d.unwrap().as_ptr()).expand(new_size as usize) {
                    Ok(_) => {}
                    Err(_) => return false,
                }
            }
        }
        true
    }

    pub fn get_fair_random_dict_index(&self) -> usize {
        let target = if self.kvstore_size() > 0 {
            rand::rng().random::<u64>() % self.kvstore_size() + 1
        } else {
            0
        };
        self.find_dict_index_by_key_index(target)
    }

    pub fn get_next_non_empty_dict_index(&self, didx: usize) -> i32 {
        if self.num_dicts == 1 {
            assert_eq!(didx, 0);
            return -1;
        }
        let next_key = self.cumulative_key_count_read(didx as i32) + 1;
        if next_key < self.kvstore_size() {
            self.find_dict_index_by_key_index(next_key) as i32
        } else {
            -1
        }
    }

    pub fn get_first_non_empty_dict_index(&self) -> usize {
        self.find_dict_index_by_key_index(1)
    }

    fn cumulative_key_count_read(&self, didx: i32) -> u64 {
        if self.num_dicts == 1 {
            assert_eq!(0, didx);
            return self.kvstore_size();
        }
        let mut idx = didx + 1;
        let mut sum = 0;
        while idx > 0 {
            sum += self.dict_size_index[idx as usize];
            idx -= idx & -idx;
        }
        sum
    }

    fn cumulative_key_count_add(&mut self, didx: i32, delta: i64) {
        unsafe {
            self.key_count = (self.key_count as i64 + delta) as u64;
            let d = self.get_dict(didx as usize);
            let dsize = (*d.unwrap().as_ptr()).dict_size();
            let non_empty_dicts_delta = if dsize == 1 && delta > 0 {
                1
            } else {
                if dsize == 0 {
                    -1
                } else {
                    0
                }
            };
            self.non_empty_dicts += non_empty_dicts_delta;

            if self.num_dicts == 1 {
                return;
            }
            let mut idx = didx + 1;
            while idx <= self.num_dicts as i32 {
                if delta < 0 {
                    assert!(self.dict_size_index[idx as usize] >= delta.abs() as u64);
                }
                self.dict_size_index[idx as usize] =
                    (self.dict_size_index[idx as usize] as i64 + delta) as u64;
                idx += idx & -idx;
            }
        }
    }

    pub fn find_dict_index_by_key_index(&self, mut target: u64) -> usize {
        if self.num_dicts == 1 || self.kvstore_size() == 0 {
            return 0;
        }
        assert!(target < self.kvstore_size());

        let mut result = 0;
        let bit_mask = 1 << self.num_dicts_bits;
        let mut i = bit_mask;
        while i != 0 {
            let current = result + i;
            if target > self.dict_size_index[current] {
                target -= self.dict_size_index[current];
                result = current;
            }
            i >>= 1;
        }
        result
    }

    fn add_dict_index_to_cursor(&self, didx: i32, cursor: &mut u64) {
        if self.num_dicts == 1 {
            return;
        }
        if didx < 0 {
            return;
        }
        *cursor = *cursor << self.num_dicts_bits | didx as u64;
    }

    pub fn iter(&mut self) -> KvStoreIterator<V> {
        unsafe {
            let mut dict = self.get_dict(0).unwrap();
            let dict_iter = DictIterMut::new(&mut *dict.as_mut());
            let next_didx = self.get_first_non_empty_dict_index() as i32;

            KvStoreIterator {
                kvs: self,
                didx: 0,
                next_didx,
                di: dict_iter,
            }
        }
    }

    pub fn try_resize_dicts(&mut self, mut limit: i32) {
        if limit > self.num_dicts as i32 {
            limit = self.num_dicts as i32;
        }
        unsafe {
            for _ in 0..limit {
                let didx = self.resize_cursor;
                let d = self.get_dict(didx as usize);
                if d.is_some() && (*d.unwrap().as_ptr()).shrink_if_needed().unwrap() != true {
                    (*d.unwrap().as_ptr()).expand_if_needed().unwrap();
                }
                self.resize_cursor = (didx + 1) % self.num_dicts as i32;
            }
        }
    }

    pub fn increment_rehash(&self, threshold_us: u64) -> u64 {
        if self.rehashing.length() == 0 {
            return 0;
        }
        let mut elapsed_us = 0;
        let start = Instant::now();
        unsafe {
            while self.rehashing.list_first().is_some() {
                let mut node = self.rehashing.list_first();
                let _ = (*(*node.as_mut().unwrap().as_ptr()).value().as_ptr())
                    .rehash_microseconds(threshold_us - elapsed_us);
                elapsed_us = start.elapsed().as_secs();
                if elapsed_us > threshold_us {
                    break;
                }
            }
        }
        elapsed_us
    }

    pub fn kvstore_overhead_hashtable_lut(&self) -> usize {
        self.overhead_hashtable_lut * size_of::<DictEntry<V>>()
    }

    pub fn kvstore_overhead_hashtable_rehashing(&self) -> usize {
        self.overhead_hashtable_rehashing * size_of::<DictEntry<V>>()
    }

    pub fn dict_rehashing_count(&self) -> usize {
        self.rehashing.length()
    }

    pub fn dict_size(&self, didx: usize) -> u64 {
        let d = self.get_dict(didx);
        if d.is_none() {
            return 0;
        }
        unsafe { (*d.unwrap().as_ptr()).dict_size() as u64 }
    }

    pub fn get_dict_iterator(&mut self, didx: usize) -> KvStoreDictIterator<V> {
        unsafe {
            let d = self.dicts[didx];
            let dict_iter = (*d.unwrap().as_ptr()).iter_mut();
            let iter = KvStoreDictIterator {
                kvs: self,
                didx: didx as i32,
                di: dict_iter,
            };
            iter
        }
    }

    pub fn get_dict_safe_iterator(&mut self, didx: usize) -> KvStoreDictIterator<V> {
        unsafe {
            let d = self.dicts[didx];
            let dict_iter = (*d.unwrap().as_ptr()).safe_iter_mut();
            KvStoreDictIterator {
                kvs: self,
                didx: didx as i32,
                di: dict_iter,
            }
        }
    }

    pub fn get_random_key(&self, didx: i32) -> Option<NonNull<DictEntry<V>>> {
        unsafe {
            let d = self.get_dict(didx as usize);
            if d.is_none() {
                return None;
            }
            (*d.unwrap().as_ptr()).get_random_key()
        }
    }

    pub fn get_fair_random_key(&self, didx: i32) -> Option<NonNull<DictEntry<V>>> {
        unsafe {
            let d = self.get_dict(didx as usize);
            if d.is_none() {
                return None;
            }
            (*d.unwrap().as_ptr()).get_fair_random_key()
        }
    }

    pub fn dict_find(&self, didx: i32, key: &str) -> Option<NonNull<DictEntry<V>>> {
        let d = self.get_dict(didx as usize);
        if let Some(d) = d {
            unsafe { (*d.as_ptr()).find(key) }
        } else {
            None
        }
    }

    pub fn dict_add_raw(&mut self, didx: i32, key: String) -> Option<NonNull<DictEntry<V>>> {
        unsafe {
            let d = self.create_dict_if_needed(didx);
            if let Ok(ret) = (*d.unwrap().as_ptr()).add_raw_without_value(key) {
                self.cumulative_key_count_add(didx, 1);
                return Some(ret);
            }
            None
        }
    }

    pub fn add(&mut self, didx: i32, key: String, val: V) -> Option<NonNull<DictEntry<V>>> {
        unsafe {
            let d = self.create_dict_if_needed(didx);
            if let Ok(ret) = (*d.unwrap().as_ptr()).add_raw(key, val) {
                self.cumulative_key_count_add(didx, 1);
                return Some(ret);
            }
            None
        }
    }

    pub fn dict_set_key(&mut self, didx: i32, old_key: &str, new_key: String) {
        unsafe {
            let d = self.dict_find(didx, &old_key);
            let mut old = (*d.unwrap().as_ptr()).get_val();
        }
    }

    pub fn dict_set_val(&mut self, didx: i32, key: &str, val: RedisObject<String>) {
        unsafe {
            let d = self.dict_find(didx, key);
            let old = &mut *((*d.unwrap().as_ptr()).get_val() as *mut V as *mut RedisObject<String>);
            *old = val;
        }
    }

    pub fn dict_two_phase_unlink_free(&mut self, didx: i32, he: Option<NonNull<DictEntry<V>>>, plink: Option<NonNull<DictEntry<V>>>, table_index: usize, ) {
        unsafe {
            let d = self.get_dict(didx as usize);
            (*d.unwrap().as_ptr()).dict_two_phase_unlink_free(he, plink, table_index);
            self.cumulative_key_count_add(didx, -1);
            self.free_dict_if_needed(didx as usize);
        }
    }

    pub fn dict_delete(&mut self, didx: i32, key: &str) -> Option<NonNull<DictEntry<V>>> {
        unsafe {
            let d = self.get_dict(didx as usize);
            if d.is_none() {
                return None;
            }
            let ret = (*d.unwrap().as_ptr()).generic_delete(key);
            match ret {
                Ok(ret) => {
                    self.cumulative_key_count_add(didx, -1);
                    self.free_dict_if_needed(didx as usize);
                    ret
                }
                Err(_) => None,
            }
        }
    }

    pub fn non_empty_dicts(&self) -> i32 {
        self.non_empty_dicts
    }
}

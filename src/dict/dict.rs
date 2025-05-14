use std::cmp::{ PartialEq};
use std::ptr::NonNull;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::mem;
use std::time::{Instant};

use crate::dict::lib::{DICT_CAN_RESIZE, DICT_FORCE_RESIZE_RATIO, DICT_HT_INITIAL_EXP, DICT_HT_INITIAL_SIZE, DictResizeFlag, HASHTABLE_MIN_FILL};
use crate::dict::lib::DictResizeFlag::DictResizeForbid;
use crate::dict::error::HashError;
use crate::dict::hash::{sys_hash};
use crate::dict::lib::{*};

#[derive(Debug, Copy)]
pub struct DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub(crate) key: K,
    pub(crate) val: V,
    pub(crate) next: Option<NonNull<DictEntry<K, V>>>,
}

impl<K, V> Default for DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    fn default() -> Self {
        Self {
            key: K::default(),
            val: V::default(),
            next: None,
        }
    }
}

impl <K, V> Clone for DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone 
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            val: self.val.clone(),
            next: self.next.clone(),
        }
    }
}

impl <K, V> PartialEq for DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.val == other.val
    }
}

impl <K, V> DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone 
{
    pub fn new(key: K, val: V) -> Self {
        Self {
            key,
            val,
            next: None,
        }
    }

    #[inline]
    pub fn push_back(&mut self, entry: DictEntry<K, V>) {
        unsafe {
            self.next = Some(NonNull::new_unchecked(Box::into_raw(Box::new(entry))));
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.key == K::default() && self.val == V::default()
    }

    #[inline]
    pub fn get_key(&self) -> &K {
        &self.key
    }

    #[inline]
    pub fn get_val(&self) -> &V {
        &self.val
    }

    #[inline]
    pub fn set_key(&mut self, key: K) {
        self.key = key
    }

    #[inline]
    pub fn set_val(&mut self, val: V) {
        self.val = val
    }

    #[inline]
    pub fn get_next(&mut self) -> Option<NonNull<DictEntry<K, V>>> {
        self.next
    }

    #[inline]
    pub fn set_next(&mut self, next: &mut DictEntry<K, V>) {
        unsafe {
            self.next = Some(NonNull::new_unchecked(next as *mut DictEntry<K, V>))
        }
    }

    #[inline]
    fn set_next_none(&mut self) {
        self.next = None
    }
}

#[derive(Clone)]
pub struct Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone 
{
    /// dict table
    pub ht_table: Vec<Vec<Option<NonNull<DictEntry<K, V>>>>>,
    /// dict table used
    ht_used: Vec<u32>,
    /// rehashing not in progress if rehash_idx == -1
    rehash_idx: i64,
    /// If >0 rehashing is paused
    pause_rehash: u64,
    /// If >0 automatic resizing is disallowed (<0 indicates coding error)
    pause_auto_resize: u16,
    /// exponent of size. (size = 1<<exp)
    pub ht_size_exp: Vec<i32>,
}

impl <K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash + Display,
      V: Default + PartialEq + Clone 
{
    pub fn new() -> Self {
        unsafe {
            Self {
                ht_table: vec![vec![Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry::default())))); DICT_HT_INITIAL_SIZE], vec![]],
                ht_used: vec![0; 2],
                rehash_idx: -1,
                pause_rehash: 0,
                pause_auto_resize: 0,
                ht_size_exp: vec![DICT_HT_INITIAL_EXP as i32; 2],
            }
        }
    }

    pub unsafe fn find_position_for_insert(&mut self, key: &K) -> Option<NonNull<DictEntry<K, V>>> {
        let hash = sys_hash(&key);
        let mut idx = hash & dict_size_mask(self.ht_size_exp[0]);
        //Rehash the dict table if needed
        self.rehash_step_if_needed(idx);
        let _ = self._expand_if_needed();

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx { continue; }
            idx = hash & dict_size_mask(self.ht_size_exp[table]);
            // Search if this slot does not already contain the given key
            let mut he= self.ht_table[table][idx as usize];
            while he.is_some() {
                let he_key = (*he.unwrap().as_ptr()).get_key();
                if *key == *he_key {
                    return None;
                }
                he = (*he.unwrap().as_ptr()).next;
            }
            if !self.dict_is_rehashing() { break; }
        }
        let ht_idx = if self.dict_is_rehashing() { 1 } else { 0 };
        let mut bucket = self.ht_table[ht_idx][idx as usize];
        bucket
    }

    pub unsafe fn add_raw(&mut self, key: K, val: V) -> Result<bool, HashError> {
        let hash = sys_hash(&key);
        let mut idx = hash & dict_size_mask(self.ht_size_exp[0]);

        //Rehash the dict table if needed
        self.rehash_step_if_needed(idx);
        self._expand_if_needed()?;

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx { continue; }
            idx = hash & dict_size_mask(self.ht_size_exp[table]);
            // Search if this slot does not already contain the given key
            let mut he= self.ht_table[table][idx as usize];
            while he.is_some() {
                let he_key = (*he.unwrap().as_ptr()).get_key();
                if key == *he_key {
                    return Ok(false);
                }
                he = (*he.unwrap().as_ptr()).next;
            }
            if !self.dict_is_rehashing() { break; }
        }

        let ht_idx: usize = if self.dict_is_rehashing() { 1 } else { 0 };
        let entry = NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry {
            key,
            val,
            next: self.ht_table[ht_idx][idx as usize],
        })));
        self.ht_table[ht_idx][idx as usize] = Some(entry);
        self.ht_used[ht_idx] += 1;
        Ok(true)
    }

    unsafe fn find_by_hash(&mut self, key: &K, hash: u64) -> Option<NonNull<DictEntry<K, V>>> {
        if self.dict_size() == 0 {
            return None;
        }
        let mut idx = hash & dict_size_mask(self.ht_size_exp[0]);
        self.rehash_step_if_needed(idx);

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            idx = hash & dict_size_mask(self.ht_size_exp[table]);

            let mut he = self.ht_table[table][idx as usize];
            while *he.unwrap().as_ptr() != DictEntry::default() {
                let he_key = (*he.unwrap().as_ptr()).get_key();
                if he_key == key {
                    return he;
                }
                he = (*he.unwrap().as_ptr()).next;
            }
        }
        None
    }

    pub unsafe fn find(&mut self, key: &K) -> Option<NonNull<DictEntry<K, V>>> {
        if self.dict_size() == 0 {
            return None;
        }

        let hash = sys_hash(key);
        self.find_by_hash(key, hash)
    }

    pub unsafe fn generic_delete(&mut self, key: &K) {
        if self.dict_size() == 0 {
            return;
        }
        let h = sys_hash(key);
        let mut idx = h & dict_size_mask(self.ht_size_exp[0]);

        self.rehash_step_if_needed(idx);

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            idx = h & dict_size_mask(self.ht_size_exp[table]);
            let mut he = self.ht_table[table][idx as usize];
            let mut prev_he = Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry::default()))));
            while *he.unwrap().as_ptr() != DictEntry::default() {
                let next_de = (*he.unwrap().as_ptr()).next;
                let he_key = (*he.unwrap().as_ptr()).get_key();
                if *key == *he_key {
                    if *prev_he.unwrap().as_ptr() != DictEntry::default() {
                        (*prev_he.unwrap().as_ptr()).next = next_de;
                    } else {
                        self.ht_table[table][idx as usize] = (*he.unwrap().as_ptr()).next;
                    }
                    self.ht_used[table] -= 1;
                    return;
                }
                prev_he = he;
                he = next_de;
            }
            if !self.dict_is_rehashing() { break; }
        }
        return
    }

    pub fn rehash_entries_in_bucket_at_index(&mut self, idx: u64) {
        unsafe {
            let mut de = self.ht_table[0][idx as usize];
            let mut h = 0;
            while *de.unwrap().as_ptr() != DictEntry::default() {
                let next_de = (*de.unwrap().as_ptr()).next;
                if self.ht_size_exp[1] > self.ht_size_exp[0] {
                    h = sys_hash((*de.unwrap().as_ptr()).get_key()) & dict_size_mask(self.ht_size_exp[1]);
                } else {
                    // shrinking the table.
                    h = idx & dict_size_mask(self.ht_size_exp[1]);
                }
                (*de.unwrap().as_ptr()).next = self.ht_table[1][h as usize];
                self.ht_table[1][h as usize] = de;
                self.ht_used[0] -= 1;
                self.ht_used[1] += 1;
                de = next_de;
            }
            self.ht_table[0][idx as usize] = Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry::default()))));
        }
    }

    fn bucket_rehash(&mut self, idx: u64) -> bool {
        unsafe {
            if self.pause_rehash != 0 {
                return false;
            }
            let s0 = dict_size(self.ht_size_exp[0]);
            let s1 = dict_size(self.ht_size_exp[1]);
            if DICT_CAN_RESIZE == DictResizeForbid || !self.dict_is_rehashing() {
                return false;
            }

            if DICT_CAN_RESIZE == DictResizeFlag::DictResizeAvoid && ((s1 > s0 && s1 < DICT_FORCE_RESIZE_RATIO * s0) || (s1 < s0 && s0 < HASHTABLE_MIN_FILL * DICT_FORCE_RESIZE_RATIO * s1)) {
                return false;
            }
            self.rehash_entries_in_bucket_at_index(idx);
            self.check_rehashing_complete();
            true
        }
    }

    fn check_rehashing_complete(&mut self) -> bool {
        if self.ht_used[0] != 0 { return false; }

        unsafe {
            self.ht_table[0] = mem::replace(&mut self.ht_table[1], vec![]);
        }
        self.ht_used[0] = self.ht_used[1];
        self.ht_size_exp[0] = self.ht_size_exp[1];
        self.reset(1);
        self.rehash_idx = -1;
        true
    }

    pub unsafe fn rehash(&mut self, mut n: usize) -> Result<bool, HashError> {
        let mut empty_visits = n * 10;
        let s0 = dict_size(self.ht_size_exp[0]);
        let s1 = dict_size(self.ht_size_exp[1]);

        if DICT_CAN_RESIZE == DictResizeForbid || !self.dict_is_rehashing() {
            return Err(HashError::RehashErr("rehash forbid or is rehashing".to_string()));
        }
        // If dict_can_resize is DICT_RESIZE_AVOID, we want to avoid rehashing.
        // If expanding, the threshold is dict_force_resize_ratio which is 4.
        // If shrinking, the threshold is 1 / (HASHTABLE_MIN_FILL * dict_force_resize_ratio) which is 1/32.
        if DICT_CAN_RESIZE == DictResizeFlag::DictResizeAvoid && ((s1 > s0 && s1 < DICT_FORCE_RESIZE_RATIO * s0) || (s1 < s0 && s0 < HASHTABLE_MIN_FILL * DICT_FORCE_RESIZE_RATIO * s1)) {
            return Err(HashError::RehashErr("rehash avoid".to_string()));
        }

        loop {
            if n == 0 || self.ht_used[0] == 0 {
                break;
            }
            assert!(dict_size(self.ht_size_exp[0]) > self.rehash_idx as u64);
            while self.ht_table[0][self.rehash_idx as usize] == None {
                self.rehash_idx += 1;
                empty_visits -= 1;
                if empty_visits == 0 {
                    return Ok(true);
                }
            }
            // Move all the keys in this bucket from the old to the new dict HT
            self.rehash_entries_in_bucket_at_index(self.rehash_idx as u64);
            self.rehash_idx += 1;
            n -= 1;
        }

        Ok(!self.check_rehashing_complete())
    }

    pub unsafe fn rehash_step(&mut self) -> Result<(), HashError> {
        if self.pause_rehash == 0 {
            self.rehash(1)?;
        }
        Ok(())
    }

    pub unsafe fn rehash_step_if_needed(&mut self, visited_index: u64) {
        if !self.dict_is_rehashing() || self.pause_rehash != 0 {
            return;
        }
        // rehashing not in progress if rehash_idx == -1
        if visited_index as i64 >= self.rehash_idx && (*self.ht_table[0][visited_index as usize].unwrap().as_ptr()) != DictEntry::default() {
            // If we have a valid dict entry at `idx` in ht0, we perform rehash on the bucket at `idx` (being more CPU cache friendly)
            self.bucket_rehash(visited_index);
        } else {
            // If the dict entry is not in ht0, we rehash the buckets based on the rehashidx (not CPU cache friendly)
            let _ = self.rehash(1);
        }
    }

    fn resize(&mut self, size: usize) -> Result<(), HashError> {
        assert!(!self.dict_is_rehashing());
        let new_ht_size_exp = next_exp(size);
        let new_size = dict_size(new_ht_size_exp);

        if new_ht_size_exp == self.ht_size_exp[0] {
            return Err(HashError::RehashErr(format!("old dict size: {} is equal to new dict size:{}", self.ht_used[0], new_ht_size_exp)));
        }
        unsafe {
            let new_ht_table = vec![Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry::default())))); new_size as usize];
            self.ht_size_exp[1] = new_ht_size_exp;
            self.ht_used[1] = 0;
            self.ht_table[1] = new_ht_table;
            self.rehash_idx = 0;

            if self.ht_table[0].is_empty() || self.ht_used[0] == 0 {
                self.ht_size_exp[0] = new_ht_size_exp;
                self.ht_used[0] = 0;
                self.ht_table[1] = vec![Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry::default())))); new_size as usize];
                self.reset(1);
                self.rehash_idx = -1;
                return Ok(());
            }
        }

        Ok(())
    }

    fn expand(&mut self, size: usize) -> Result<(), HashError> {
        if self.dict_is_rehashing() || self.ht_used[0] > (size as u32) || dict_size(self.ht_size_exp[0]) >= (size as u64) {
            return Err(HashError::ExpandErr("size is invalid".to_string()));
        }
        self.resize(size)
    }

    fn expand_if_needed(&mut self) -> Result<bool, HashError> {
        if self.dict_is_rehashing() {
            return Ok(true);
        }

        if dict_size(self.ht_size_exp[0]) == 0 {
            self.expand(DICT_HT_INITIAL_SIZE)?;
            return Ok(true);
        }

        let ht_used = self.ht_used[0] as u64;
        unsafe {
            if DICT_CAN_RESIZE == DictResizeFlag::DictResizeEnable && ht_used >= dict_size(self.ht_size_exp[0]) || (DICT_CAN_RESIZE != DictResizeForbid && ht_used >= DICT_FORCE_RESIZE_RATIO * dict_size(self.ht_size_exp[0])) {
                self.expand((ht_used + 1) as usize)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn _expand_if_needed(&mut self) -> Result<bool, HashError> {
        if self.pause_auto_resize > 0 {
            return Ok(true);
        }
        self.expand_if_needed()
    }

    pub fn pause_rehash(&mut self) {
        self.pause_rehash += 1
    }

    pub fn rehash_microseconds(&mut self, us: u64) -> Result<i32, HashError> {
        if self.pause_rehash > 0 {
            return Ok(0);
        }

        let start = Instant::now();
        let mut rehashes = 0;
        unsafe {
            while self.rehash(100)? {
                rehashes += 100;
                if start.elapsed().as_micros() as u64 >= us {
                    break;
                }
            }
        }
        Ok(rehashes)
    }
}

impl <K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone 
{
    #[inline]
    pub fn reset(&mut self, table: usize) {
        self.ht_table[table] = vec![];
        self.ht_used[table] = 0;
        self.ht_size_exp[table] = -1;
        self.rehash_idx = -1;
    }

    #[inline]
    pub unsafe fn init(&mut self) {
        self.reset(0);
        self.reset(1);
    }

    #[inline]
    pub fn dict_is_rehashing(&self) -> bool {
        self.rehash_idx != -1
    }

    #[inline]
    pub fn get_rehash_idx(&self) -> i64 {
        self.rehash_idx
    }

    #[inline]
    pub fn dict_buckets(&self) -> u64 {
        let size0 = self.ht_size_exp[0];
        let size1 = self.ht_size_exp[1];
        dict_size(size0) + dict_size(size1)
    }

    #[inline]
    pub fn dict_size(&self) -> u32 {
        self.ht_used[0] + self.ht_used[1]
    }

    #[inline]
    fn dict_is_empty(&self) -> bool {
        self.ht_used[0] == 0 && self.ht_used[1] == 0
    }

    #[inline]
    fn dict_pause_rehash(&mut self) {
        self.pause_rehash += 1
    }

    #[inline]
    fn dict_resume_rehash(&mut self) {
        self.pause_rehash -= 1
    }

    #[inline]
    fn dict_is_rehash_paused(&self) -> bool {
        self.pause_rehash > 0
    }
}


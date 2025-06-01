use std::any::Any;
use std::cmp::{ PartialEq};
use std::ptr::NonNull;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::mem;
use std::sync::Arc;
use std::time::{Instant};
use rand::{random, Rng, rng};
use crate::dict::lib::{DICT_CAN_RESIZE, DICT_FORCE_RESIZE_RATIO, DICT_HT_INITIAL_EXP, DICT_HT_INITIAL_SIZE, DictResizeFlag, HASHTABLE_MIN_FILL};
use crate::dict::lib::DictResizeFlag::{DictResizeEnable, DictResizeForbid};
use crate::dict::error::HashError;
use crate::dict::hash::{sys_hash};
use crate::dict::lib::{*};
use crate::skiplist::lib::gen_random;

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
    #[inline]
    pub fn get_key(&self) -> &K {
        &self.key
    }

    #[inline]
    pub fn get_val(&self) -> &V {
        &self.val
    }
}


pub struct Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone 
{
    pub dict_type: Arc<DictType<K, V>>,
    /// dict table
    pub ht_table: Vec<Vec<Option<NonNull<DictEntry<K, V>>>>>,
    /// dict table used
    pub ht_used: Vec<u32>,
    /// rehashing not in progress if rehash_idx == -1
    rehash_idx: i64,
    /// If >0 rehashing is paused
    pause_rehash: u64,
    /// If >0 automatic resizing is disallowed (<0 indicates coding error)
    pause_auto_resize: u16,
    /// exponent of size. (size = 1<<exp)
    pub ht_size_exp: Vec<i32>,
    pub metadata: Vec<Box<dyn Any>>,
}

impl <K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone 
{
    pub fn create(dict_type: Arc<DictType<K, V>>) -> Self {
        unsafe {
            Self {
                dict_type,
                ht_table: vec![vec![Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry::default())))); DICT_HT_INITIAL_SIZE], vec![]],
                ht_used: vec![0; 2],
                rehash_idx: -1,
                pause_rehash: 0,
                pause_auto_resize: 0,
                ht_size_exp: vec![DICT_HT_INITIAL_EXP as i32; 2],
                metadata: vec![],
            }
        }
    }

    pub fn scan(&mut self, mut v: u64, scan_fn: DictScanFunction<K, V>) -> u64 {
        let mut ht_idx0 = 0;
        let mut ht_idx1 = 0;
        let mut m0 = 0;
        let mut m1 = 0;

        if self.dict_size() == 0 {
            return 0;
        }
        self.pause_rehash();
        unsafe {
            if !self.dict_is_rehashing() {
                m0 = dict_size_mask(self.ht_size_exp[ht_idx0]);
                let mut de = self.ht_table[ht_idx0][(v & m0) as usize];
                while de.is_some() {
                    let next = (*de.unwrap().as_ptr()).next;
                    scan_fn(&mut *de.unwrap().as_ptr());
                    de = next;
                }
                v |= !m0;
                v = v.reverse_bits();
                v += 1;
                v = v.reverse_bits();
            } else {
                ht_idx0 = 0;
                ht_idx1 = 1;

                if dict_size(self.ht_size_exp[ht_idx0]) > dict_size(self.ht_size_exp[ht_idx1]) {
                    ht_idx0 = 1;
                    ht_idx1 = 0;
                }
                m0 = dict_size_mask(self.ht_size_exp[ht_idx0]);
                m1 = dict_size_mask(self.ht_size_exp[ht_idx1]);

                let mut de = self.ht_table[ht_idx0][(v & m0) as usize];
                while de.is_some() {
                    let next = (*de.unwrap().as_ptr()).next;
                    scan_fn(&mut *de.unwrap().as_ptr());
                    de = next;
                }

                loop {
                    let mut de = self.ht_table[ht_idx1][(v & m1) as usize];
                    while de.is_some() {
                        let next = (*de.unwrap().as_ptr()).next;
                        scan_fn(&mut *de.unwrap().as_ptr());
                        de = next;
                    }
                    v |= !m1;
                    v = v.reverse_bits();
                    v += 1;
                    v = v.reverse_bits();
                    if v & (m0 ^ m1) == 0 {
                        break;
                    }
                }
            }
            self.resume_rehash();
        }
        v
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

    #[inline]
    pub fn add_raw(&mut self, key: K, val: V) -> Result<NonNull<DictEntry<K, V>>, HashError> {
        unsafe {
            let hash = sys_hash(&key);
            let mut idx = hash & dict_size_mask(self.ht_size_exp[0]);

            //Rehash the dict table if needed
            self.rehash_step_if_needed(idx);
            self._expand_if_needed()?;

            for table in 0..2 {
                if table == 0 && (idx as i64) < self.rehash_idx { continue; }
                idx = hash & dict_size_mask(self.ht_size_exp[table]);
                // Search if this slot does not already contain the given key
                let mut he = self.ht_table[table][idx as usize];
                while he.is_some() {
                    let he_key = (*he.unwrap().as_ptr()).get_key();
                    if key == *he_key {
                        return Err(HashError::DictEntryDup);
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
            Ok(entry)
        }
    }

    pub fn add_non_exists_by_hash(&mut self, key: K, hash: u64) {
        unsafe {
            let mut idx = hash & dict_size_mask(self.ht_size_exp[0]);

            self.rehash_step_if_needed(idx);
            let _ = self.expand_if_needed();

            let table = if self.dict_is_rehashing() {1} else {0};
            idx = hash & dict_size_mask(self.ht_size_exp[table]);
            let position = self.ht_table[table][idx as usize];
            assert!(position.is_some());
            let entry = NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry {
                key,
                val: V::default(),
                next: position,
            })));
            self.ht_table[table][idx as usize] = Some(entry);
            self.ht_used[table] += 1;
        }
    }

    pub fn find_by_hash(&mut self, key: &K, hash: u64) -> Option<NonNull<DictEntry<K, V>>> {
        if self.dict_size() == 0 {
            return None;
        }
        let mut idx = hash & dict_size_mask(self.ht_size_exp[0]);
        self.rehash_step_if_needed(idx);

        unsafe {
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
                if !self.dict_is_rehashing() { break; }
            }
        }
        None
    }

    pub fn find(&mut self, key: &K) -> Option<NonNull<DictEntry<K, V>>> {
        if self.dict_size() == 0 {
            return None;
        }

        let hash = sys_hash(key);
        self.find_by_hash(key, hash)
    }

    pub fn fetch_value(&mut self, key: &K) -> Option<&V> {
        let he = self.find(key);
        unsafe {
            if he.is_some() {
                return Some((*he.unwrap().as_ptr()).get_val());
            }
            None
        }
    }

    pub fn generic_delete(&mut self, key: &K) -> Result<Option<NonNull<DictEntry<K, V>>>, HashError> {
        unsafe {
            if self.dict_size() == 0 {
                return Ok(None);
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
                        self._shrink_if_expand()?;
                        return Ok(he);
                    }
                    prev_he = he;
                    he = next_de;
                }
                if !self.dict_is_rehashing() { break; }
            }
            return Ok(None)
        }
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
        self.ht_table[0] = mem::replace(&mut self.ht_table[1], vec![]);

        self.ht_used[0] = self.ht_used[1];
        self.ht_size_exp[0] = self.ht_size_exp[1];
        self.reset(1);
        self.rehash_idx = -1;
        true
    }

    pub fn rehash(&mut self, mut n: usize) -> Result<bool, HashError> {
        unsafe {
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
        }

        Ok(!self.check_rehashing_complete())
    }

    pub fn rehash_step(&mut self) -> Result<(), HashError> {
        if self.pause_rehash == 0 {
            self.rehash(1)?;
        }
        Ok(())
    }

    pub fn rehash_step_if_needed(&mut self, visited_index: u64) {
        unsafe {
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
    }

    fn resize(&mut self, size: usize) -> Result<(), HashError> {
        assert!(!self.dict_is_rehashing());

        let new_ht_size_exp = next_exp(size);
        let new_size = dict_size(new_ht_size_exp);
        if new_size < size as u64 {
            return Err(HashError::RehashErr(format!("new size: {} is less resize size:{}", new_size, size)));
        }

        if new_ht_size_exp == self.ht_size_exp[0] {
            return Err(HashError::RehashErr(format!("old dict size: {} is equal to new dict size:{}", self.ht_used[0], new_ht_size_exp)));
        }

        unsafe {
            let new_ht_table = vec![Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry::default())))); new_size as usize];
            let new_ht_used = 0;
            self.ht_size_exp[1] = new_ht_size_exp;
            self.ht_used[1] = new_ht_used;
            self.ht_table[1] = new_ht_table.clone();
            self.rehash_idx = 0;

            if self.ht_table[0].is_empty() || self.ht_used[0] == 0 {
                self.ht_size_exp[0] = new_ht_size_exp;
                self.ht_used[0] = new_ht_used;
                self.ht_table[0] = new_ht_table;
                self.reset(1);
                self.rehash_idx = -1;
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn expand(&mut self, size: usize) -> Result<(), HashError> {
        if self.dict_is_rehashing() || self.ht_used[0] > (size as u32) || dict_size(self.ht_size_exp[0]) >= (size as u64) {
            return Err(HashError::ExpandErr("size is invalid".to_string()));
        }
        self.resize(size)
    }

    pub fn expand_if_needed(&mut self) -> Result<bool, HashError> {
        if self.dict_is_rehashing() {
            return Ok(true);
        }

        if dict_size(self.ht_size_exp[0]) == 0 {
            self.expand(DICT_HT_INITIAL_SIZE)?;
            return Ok(true);
        }

        let ht_used = self.ht_used[0] as u64;
        unsafe {
            if DICT_CAN_RESIZE == DictResizeEnable && ht_used >= dict_size(self.ht_size_exp[0]) || (DICT_CAN_RESIZE != DictResizeForbid && ht_used >= DICT_FORCE_RESIZE_RATIO * dict_size(self.ht_size_exp[0])) {
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

    pub fn _shrink_if_expand(&mut self) -> Result<bool, HashError> {
        if self.pause_auto_resize > 0 {
            return Ok(false);
        }
        self.shrink_if_needed()
    }

    pub fn shrink_if_needed(&mut self) -> Result<bool, HashError> {
        if self.dict_is_rehashing() {
            return Ok(true);
        }
        if dict_size(self.ht_size_exp[0]) <= DICT_HT_INITIAL_SIZE as u64 {
            return Ok(true);
        }

        unsafe {
            if (DICT_CAN_RESIZE == DictResizeEnable && self.ht_used[0] as u64 * HASHTABLE_MIN_FILL <= dict_size(self.ht_size_exp[0])) || (DICT_CAN_RESIZE != DictResizeForbid && self.ht_used[0] as u64 * HASHTABLE_MIN_FILL * DICT_FORCE_RESIZE_RATIO <= dict_size(self.ht_size_exp[0])) {
                self.shrink(self.ht_used[0] as u64)?;
                return Ok(true)
            }
        }
        Ok(false)
    }

    pub fn shrink(&mut self, size: u64) -> Result<(), HashError> {
        if self.dict_is_rehashing() || (self.ht_used[0] as u64 > size) || (dict_size(self.ht_size_exp[0]) <= size) {
            return Err(HashError::ShrinkErr(-2));
        }
        self.resize(size as usize)
    }

    pub fn pause_rehash(&mut self) {
        self.pause_rehash += 1
    }

    pub fn resume_rehash(&mut self) {
        self.pause_rehash -= 1
    }

    pub fn pause_auto_resize(&mut self) {
        self.pause_auto_resize += 1
    }

    pub fn resume_auto_resize(&mut self) {
        self.pause_auto_resize -= 1
    }

    pub fn is_rehash_pause(&self) -> bool {
        self.pause_rehash > 0
    }

    pub fn rehash_microseconds(&mut self, us: u64) -> Result<i32, HashError> {
        if self.pause_rehash > 0 {
            return Ok(0);
        }

        let start = Instant::now();
        let mut rehashes = 0;
        while self.rehash(100)? {
            rehashes += 100;
            if start.elapsed().as_micros() as u64 >= us {
                break;
            }
        }

        Ok(rehashes)
    }

    pub fn release(&mut self) {
        self._clear(0, None);
        self._clear(1, None);
    }

    fn _clear(&mut self, ht_idx: usize, call_back: Option<fn(&mut Dict<K, V>)>) {
        unsafe {
            for i in 0..dict_size(self.ht_size_exp[ht_idx]) {
                if self.ht_used[ht_idx] <= 0 {
                    break;
                }
                if call_back.is_some() && i != 0 && (i & 65535) == 0 {
                    call_back.unwrap();
                }
                let mut he = self.ht_table[ht_idx][i as usize];
                if *he.unwrap().as_ptr() == DictEntry::default() { continue; }

                while *he.unwrap().as_ptr() != DictEntry::default() {
                    let box_node = Box::from_raw(he.unwrap().as_ptr());
                    he = box_node.next;
                    self.ht_used[ht_idx] -= 1;
                }
            }
            self.reset(ht_idx);
        }
    }

    pub fn empty(&mut self, call_back: Option<fn(&mut Dict<K, V>)>) {
        self._clear(0, call_back);
        self._clear(1, call_back);

        self.rehash_idx = -1;
        self.pause_rehash = 0;
        self.pause_auto_resize = 0;
    }

    pub fn get_random_key(&mut self) -> Option<NonNull<DictEntry<K ,V>>> {
        unsafe {
            let mut he;
            if self.dict_size() == 0 {
                return None;
            }
            if self.dict_is_rehashing() {
                let _ = self.rehash_step();
            }
            if self.dict_is_rehashing() {
                let s0 = dict_size(self.ht_size_exp[0]) as i64;
                // We are sure there are no elements in indexes from 0 to rehashidx-1
                loop {
                    let h = self.rehash_idx + random_ulong() as i64 % (self.dict_buckets() as i64 - self.rehash_idx);
                    he = if h >= s0 {
                        self.ht_table[1][(h - s0) as usize]
                    } else {
                        self.ht_table[0][h as usize]
                    };
                    if *he.unwrap().as_ptr() != DictEntry::default() {
                        break;
                    }
                }
            } else {
                let m = dict_size_mask(self.ht_size_exp[0]);
                loop {
                    let h = random_ulong() & m;
                    he = self.ht_table[0][h as usize];
                    if *he.unwrap().as_ptr() != DictEntry::default() {
                        break;
                    }
                }
            }

            let mut list_len = 0;
            let orig_he = he;
            while he.is_some() {
                he = (*he.unwrap().as_ptr()).next;
                list_len += 1;
            }
            let mut list_ele = random_u32() % list_len;
            he = orig_he;

            while list_ele > 0 {
                he = (*he.unwrap().as_ptr()).next;
                list_ele -= 1;
            }
            he
        }
    }

    pub fn get_fair_random_key(&mut self) -> Option<NonNull<DictEntry<K ,V>>>{
        let mut entries = Vec::with_capacity(GETFAIR_NUM_ENTRIES);
        let count = GETFAIR_NUM_ENTRIES;
        let cnt = self.get_some_keys(&mut entries, count as u64);

        if cnt == 0 {
            return self.get_random_key()
        }
        let idx = gen_random() % count as u32 ;
        entries[idx as usize]
    }

    fn get_some_keys(&mut self, des: &mut Vec<Option<NonNull<DictEntry<K, V>>>>, mut count: u64) -> u64 {
        let mut stored = 0;
        if (self.dict_size() as u64) < count {
            count = self.dict_size() as u64;
        }
        let mut max_step = count * 10;
        for j in 0..count {
            if self.dict_is_rehashing() {
                let _ = self.rehash_step();
            } else {
                break;
            }
        }

        let table = if self.dict_is_rehashing() {2} else {1};
        let mut max_size_mask = dict_size_mask(self.ht_size_exp[0]);
        if table > 1 && max_size_mask < dict_size_mask(self.ht_size_exp[1]) {
            max_size_mask = dict_size_mask(self.ht_size_exp[1]);
        }

        let mut i = random_ulong() & max_size_mask;
        let mut empty_len = 0;
        unsafe {
            while stored < count && max_step > 0 {
                max_step -= 1;
                for j in 0..table {
                    if table == 2 && j == 0 && i < self.rehash_idx as u64 {
                        if i >= dict_size(self.ht_size_exp[1]) {
                            i = self.rehash_idx as u64;
                        } else {
                            continue;
                        }
                    }
                    if i >= dict_size(self.ht_size_exp[j]) { continue; }
                    let mut he = self.ht_table[j][i as usize];

                    if he.is_none() {
                        empty_len += 1;
                        if empty_len >= 5 && empty_len > count {
                            i = random_ulong() & max_size_mask;
                            empty_len = 0;
                        }
                    } else {
                        empty_len = 0;
                        while he.is_some() {
                            if stored < count {
                                des[stored as usize] = he;
                            } else {
                                let r = random_ulong() % (stored + 1);
                                if r < count {
                                    des[r as usize] = he;
                                }
                            }
                            he = (*he.unwrap().as_ptr()).next;
                            stored += 1;
                        }
                        if stored >= count {
                            break;
                        }
                    }
                }
                i = (i + 1) & max_size_mask;
            }
        }
        return if stored > count { stored } else { count }
    }

    pub fn find_by_hash_and_ptr(&self, key: K, hash: u64) -> Option<NonNull<DictEntry<K, V>>> {
        if self.dict_size() == 0 {
            return None;
        }
        unsafe {
            for table in 0..2 {
                let idx = hash & dict_size_mask(self.ht_size_exp[table]);
                if table == 0 && (idx as i64) < self.rehash_idx { continue; }
                let mut he = self.ht_table[table][idx as usize];
                while he.is_some() {
                    if key == (*he.unwrap().as_ptr()).key {
                        return he;
                    }
                    he = (*he.unwrap().as_ptr()).next;
                }
                if !self.dict_is_rehashing() {
                    return None;
                }
            }
        }
        None
    }

    pub fn rehash_info(&self, from: &mut u64, to: &mut u64) {
        assert!(self.dict_is_rehashing());
        *from = dict_size(self.ht_size_exp[0]);
        *to = dict_size(self.ht_size_exp[1]);
    }

    pub fn dict_two_phase_unlink_find(&mut self, key: &K, table_index: &mut usize) -> Option<NonNull<DictEntry<K, V>>> {
        if self.dict_size() == 0 {
            return None;
        }
        if self.dict_is_rehashing() {
            let _ = self.rehash_step();
        }
        let h = sys_hash(key);
        unsafe {
            for table in 0..2 {
                let idx = h & dict_size_mask(self.ht_size_exp[table]);
                if table == 0 && (idx as i64) < self.rehash_idx { continue; }
                let mut he = self.ht_table[table][idx as usize];
                while he.is_some() {
                    let he_key = (*he.unwrap().as_ptr()).get_key();
                    if he_key == key {
                        *table_index = table;
                        self.pause_rehash();
                        return he;
                    }
                    he = (*he.unwrap().as_ptr()).next;
                }
                if !self.dict_is_rehashing() {
                    return None;
                }
            }
        }
        None
    }

    pub fn dict_two_phase_unlink_free(
        &mut self,
        he: Option<NonNull<DictEntry<K, V>>>,
        mut plink: Option<NonNull<DictEntry<K, V>>>,
        table_index: usize
    ) {
        if he.is_none() {
            return;
        }
        unsafe {
            let box_node = Box::from_raw(he.unwrap().as_ptr());
            self.ht_used[table_index] -= 1;
            plink = (*he.unwrap().as_ptr()).next;
            let _ = self.shrink_if_needed();
            self.resume_rehash();
        }
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
    pub fn dict_is_empty(&self) -> bool {
        self.ht_used[0] == 0 && self.ht_used[1] == 0
    }

    #[inline]
    pub fn dict_pause_rehash(&mut self) {
        self.pause_rehash += 1
    }

    #[inline]
    pub fn dict_resume_rehash(&mut self) {
        self.pause_rehash -= 1
    }

    #[inline]
    pub fn dict_is_rehash_paused(&self) -> bool {
        self.pause_rehash > 0
    }
}


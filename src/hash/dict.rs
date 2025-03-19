use std::cmp::PartialEq;
use std::ptr::NonNull;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::mem;
use std::ops::Deref;

use crate::hash::{DICT_CAN_RESIZE, DICT_FORCE_RESIZE_RATIO, DICT_HT_INITIAL_EXP, DICT_HT_INITIAL_SIZE, DictResizeFlag, HASHTABLE_MIN_FILL, LONG_MAX};
use crate::hash::DictResizeFlag::DictResizeForbid;
use crate::hash::error::HashError;
use crate::hash::hash::{sys_hash};

#[inline]
pub fn dict_size(exp: i32) -> u64 {
    return if exp == -1 {
        0
    } else {
        1 << exp
    }
}

#[inline]
pub fn dict_size_mask(exp: i32) -> u64 {
    return if exp == -1 {
        0
    } else {
        dict_size(exp) - 1
    }
}

pub fn pause_rehash<K, V>(dict: &mut Dict<K, V>)
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    dict.pause_rehash += 1
}

fn next_exp(size: usize) -> i32 {
    if size <= DICT_HT_INITIAL_SIZE {
        return DICT_HT_INITIAL_EXP as i32;
    }
    let long_bits = size_of::<usize>() * 8;
    if size >= LONG_MAX as usize {
        return (long_bits - 1) as i32;
    }
    let leading_zeros = (size - 1).leading_zeros() as usize;
    (long_bits - leading_zeros) as i32
}

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

// impl <K, V> Display for DictEntry<K, V>
// where K: Default + Clone + Eq + Hash,
//       V: Default + PartialEq + Clone
// {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "key: {}, val: {}", self.key, self.val)
//     }
// }

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

    pub unsafe fn new_with_next(key: K, val: V, next: Option<NonNull<DictEntry<K, V>>>) -> Self {
        Self {
            key,
            val,
            next,
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
    pub fn get_key(&self) -> K {
        self.key.clone()
    }

    #[inline]
    pub fn get_val(&self) -> V {
        self.val.clone()
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
    pub fn get_next_ref(&self) -> Option<&mut DictEntry<K, V>> {
        unsafe {
            self.next.map(|next| &mut (*next.as_ptr()))
        }
    }

    #[inline]
    pub fn get_next(&self) -> Option<DictEntry<K, V>> {
        unsafe {
            self.next.map(|mut entry| (*entry.as_ptr()).clone())
        }
    }

    #[inline]
    pub fn set_next(&mut self, next: &DictEntry<K, V>) {
        unsafe {
            self.next = Some(NonNull::new_unchecked(Box::into_raw(Box::new(next.clone()))))
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
    /// hash table
    pub ht_table: Vec<Vec<DictEntry<K, V>>>,
    /// hash table used
    ht_used: Vec<u32>,
    /// rehashing not in progress if rehash_idx == -1
    rehash_idx: i64,
    /// If >0 rehashing is paused
    pause_rehash: u64,
    /// exponent of size. (size = 1<<exp)
    pub ht_size_exp: Vec<i32>,
}

impl <K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash + Display,
      V: Default + PartialEq + Clone 
{
    pub fn new() -> Self {
        Self {
            ht_table: vec![vec![DictEntry::default(); DICT_HT_INITIAL_SIZE], vec![]],
            ht_used: vec![0; 2],
            rehash_idx: -1,
            pause_rehash: 0,
            ht_size_exp: vec![DICT_HT_INITIAL_EXP as i32; 2],
        }
    }

    pub unsafe fn dict_add(&mut self, key: K, val: V) -> Result<(), HashError> {
        let hash = sys_hash(&key);
        let mut idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked(0));

        if self.dict_is_rehashing() {
            if (idx as i64) >= self.rehash_idx && *self.ht_table.get_unchecked(0).get_unchecked(idx as usize) != DictEntry::default() {
                self.bucket_rehash(idx as usize);
            } else {
                self.rehash_step()?;
            }
        }
        self.expand_if_needed()?;

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked(table));
            let mut head= self.ht_table.get_unchecked(table).get_unchecked(idx as usize);
            loop {
                if *head == DictEntry::default(){
                    break;
                }
                let he_key = head.get_key();
                if he_key == key {
                    return Err(HashError::DictEntryDup);
                }
                if let Some(next) = head.get_next_ref() {
                    head = next;
                } else {
                    break;
                }
            }
            if !self.dict_is_rehashing() { break; }
        }

        let ht_idx: usize = if self.dict_is_rehashing() { 1 } else { 0 };
        let bucket = self.ht_table.get_unchecked_mut(ht_idx).get_unchecked_mut(idx as usize);
        let mut entry = DictEntry::new(key, val);
        if *bucket == DictEntry::default() {
            entry.set_next_none();
        } else {
            entry.set_next(bucket);
        }
        *bucket = entry;
        *self.ht_used.get_unchecked_mut(ht_idx) += 1;

        Ok(())
    }

    pub unsafe fn dict_find(&mut self, key: &K) -> Option<&DictEntry<K, V>> {
        if self.dict_size() == 0 {
            return None;
        }

        let hash = sys_hash(key);
        let mut idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked(0));
        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked(table));
            let mut head = self.ht_table.get_unchecked(table).get_unchecked(idx as usize);
            loop {
                if *head == DictEntry::default() {
                    break;
                }
                let he_key = head.get_key();
                if he_key == *key {
                    return Some(head);
                }
                if let Some(next) = head.get_next_ref() {
                    head = next;
                } else {
                    break;
                }
            }
            if !self.dict_is_rehashing() { break; }
        }
        None
    }

    pub unsafe fn dict_delete(&mut self, key: K) -> Result<(), HashError> {
        let hash = sys_hash(&key);
        let mut idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked(0));

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked(table));
            let mut he = self.ht_table.get_unchecked_mut(table).get_unchecked_mut(idx as usize);
            let mut prev = &mut DictEntry::default();
            loop {
                if *he == DictEntry::default() {
                    break;
                }
                let he_key = he.get_key();
                if he_key == key {
                    if let Some(next) = he.get_next() {
                        if !prev.is_empty() {
                            prev.set_next(&next);
                        } else {
                            *he = next;
                        }
                    } else {
                        if !prev.is_empty() {
                            prev.set_next_none();
                        } else {
                            //println!("删除key: {}, next: None", key);
                            *he = DictEntry::default();
                        }
                    }
                    *self.ht_used.get_unchecked_mut(table) -= 1;
                    return Ok(());
                }

                //prev = he;
                if let Some(next) = he.get_next_ref() {
                    he = next;
                } else {
                    break;
                }
            }
            if !self.dict_is_rehashing() { break; }
        }
        Err(HashError::DictNoKey(key.to_string()))
    }

    pub unsafe fn rehash_at_index(&mut self, idx: usize) {
        let mut cur = mem::replace(self.ht_table.get_unchecked_mut(0).get_unchecked_mut(idx), DictEntry::default());
        loop {
            if cur == DictEntry::default() {
                break;
            }
            let next = cur.get_next();
            let key = cur.get_key();
            let mut idx = 0;
            if *self.ht_size_exp.get_unchecked(1) > *self.ht_size_exp.get_unchecked(0)
            {
                idx = sys_hash(&key) & dict_size_mask(*self.ht_size_exp.get_unchecked(1));
            }
            let target_entry = self.ht_table.get_unchecked_mut(1).get_unchecked_mut(idx as usize);
            //cur.set_next(mem::replace(&mut target_entry, &mut DictEntry::default()));
            cur.set_next(target_entry);
            *self.ht_table.get_unchecked_mut(1).get_unchecked_mut(idx as usize) = cur;
            *self.ht_used.get_unchecked_mut(0) -= 1;
            *self.ht_used.get_unchecked_mut(1) += 1;
            if let Some(next_entry) = next {
                cur = next_entry.clone();
            } else {
                break;
            }
        }
        *self.ht_table.get_unchecked_mut(0).get_unchecked_mut(idx) = DictEntry::default();
    }

    unsafe fn bucket_rehash(&mut self, idx: usize) -> bool {
        if self.pause_rehash != 0 {
            return false;
        }
        let s0 = dict_size(*self.ht_size_exp.get(0).unwrap());
        let s1 = dict_size(*self.ht_size_exp.get(1).unwrap());
        if DICT_CAN_RESIZE == DictResizeForbid || !self.dict_is_rehashing() {
            return false;
        }

        if DICT_CAN_RESIZE == DictResizeFlag::DictResizeAvoid && ((s1 > s0 && s1 < DICT_FORCE_RESIZE_RATIO * s0) || (s1 < s0 && s0 < HASHTABLE_MIN_FILL * DICT_FORCE_RESIZE_RATIO * s1)) {
            return false;
        }
        self.rehash_at_index(idx);
        self.check_rehashing_complete();
        true
    }

    unsafe fn check_rehashing_complete(&mut self) -> bool {
        if *self.ht_used.get_unchecked(0) != 0 {
            return false;
        }
        *self.ht_table.get_unchecked_mut(0) = mem::replace(&mut *self.ht_table.get_unchecked_mut(1), vec![DictEntry::default(); 0]);
        *self.ht_used.get_unchecked_mut(0) = *self.ht_used.get_unchecked(1);
        *self.ht_size_exp.get_unchecked_mut(0) = *self.ht_size_exp.get_unchecked(1);
        self.reset(1);
        self.rehash_idx = -1;
        true
    }

    pub unsafe fn rehash(&mut self, mut n: usize) -> Result<bool, HashError> {
        let mut empty_visits = n * 10;
        if DICT_CAN_RESIZE == DictResizeForbid || !self.dict_is_rehashing() {
            return Err(HashError::RehashErr("rehash forbid or is rehashing".to_string()));
        }
        let s0 = dict_size(*self.ht_size_exp.get(0).unwrap());
        let s1 = dict_size(*self.ht_size_exp.get(1).unwrap());
        if DICT_CAN_RESIZE == DictResizeFlag::DictResizeAvoid && ((s1 > s0 && s1 < DICT_FORCE_RESIZE_RATIO * s0) || (s1 < s0 && s0 < HASHTABLE_MIN_FILL * DICT_FORCE_RESIZE_RATIO * s1)) {
            return Err(HashError::RehashErr("rehash avoid".to_string()));
        }

        loop {
            if n == 0 || *self.ht_used.get_unchecked(0) == 0 {
                break;
            }
            assert!(dict_size(*self.ht_size_exp.get_unchecked(0)) as i64 > self.rehash_idx);
            loop {
                if *self.ht_table.get_unchecked(0).get_unchecked(self.rehash_idx as usize) != DictEntry::default() {
                    break;
                }
                self.rehash_idx += 1;
                empty_visits -= 1;
                if empty_visits == 0 {
                    return Ok(true);
                }
            }
            self.rehash_at_index(self.rehash_idx as usize);
            self.rehash_idx += 1;
            n -= 1;
        }
        self.check_rehashing_complete();
        Ok(true)
    }

    pub unsafe fn rehash_step(&mut self) -> Result<(), HashError> {
        if self.pause_rehash == 0 {
            self.rehash(1)?;
        }
        Ok(())
    }

    unsafe fn resize(&mut self, size: usize) -> Result<(), HashError> {
        assert!(!self.dict_is_rehashing());
        let new_ht_size_exp = next_exp(size);
        let new_ht_size = dict_size(new_ht_size_exp);

        if new_ht_size_exp == (*self.ht_used.get_unchecked_mut(0) as i32) {
            return Err(HashError::RehashErr(format!("old hash size: {} is equal to new hash size:{}", *self.ht_used.get_unchecked_mut(0), new_ht_size_exp)));
        }
        let new_ht_table = vec![DictEntry::default(); new_ht_size as usize];
        *self.ht_size_exp.get_unchecked_mut(1) = new_ht_size_exp;
        *self.ht_used.get_unchecked_mut(1) = 0;
        *self.ht_table.get_unchecked_mut(1) = new_ht_table;
        self.rehash_idx = 0;

        Ok(())
    }

    unsafe fn expand(&mut self, size: usize) -> Result<(), HashError> {
        if self.dict_is_rehashing() || *self.ht_used.get_unchecked(0) > (size as u32) || dict_size(*self.ht_size_exp.get_unchecked(0)) >= (size as u64) {
            return Err(HashError::ExpandErr("size is invalid".to_string()));
        }
        self.resize(size)
    }

    unsafe fn expand_if_needed(&mut self) -> Result<bool, HashError> {
        if self.dict_is_rehashing() {
            return Ok(true);
        }

        let ht_used = *self.ht_used.get_unchecked(0) as u64;
        if DICT_CAN_RESIZE == DictResizeFlag::DictResizeEnable && ht_used >= dict_size(*self.ht_size_exp.get_unchecked(0)) || (DICT_CAN_RESIZE != DictResizeForbid && ht_used >= DICT_FORCE_RESIZE_RATIO * dict_size(*self.ht_size_exp.get_unchecked(0))) {
            self.expand((ht_used + 1) as usize)?;
        }
        Ok(false)
    }

    pub unsafe fn get_entry(&self, table: usize, index: usize) -> DictEntry<K, V> {
        self.ht_table.get_unchecked(table).get_unchecked(index).clone()
    }

    pub fn pause_rehash(&mut self) {
        self.pause_rehash += 1
    }
}

impl <K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone 
{
    #[inline]
    pub unsafe fn reset(&mut self, table: usize) {
        *self.ht_table.get_unchecked_mut(table) = vec![];
        *self.ht_used.get_unchecked_mut(table) = 0;
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
    fn dict_buckets(&self) -> u64 {
        let size0 = *self.ht_size_exp.get(0).unwrap();
        let size1 = *self.ht_size_exp.get(1).unwrap();
        dict_size(size0) + dict_size(size1)
    }

    #[inline]
    fn dict_size(&self) -> u32 {
        *self.ht_used.get(0).unwrap() + *self.ht_used.get(1).unwrap()
    }

    #[inline]
    fn dict_is_empty(&self) -> bool {
        *self.ht_used.get(0).unwrap() == 0 && *self.ht_used.get(1).unwrap() == 0
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


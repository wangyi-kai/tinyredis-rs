use std::cmp::PartialEq;
use std::ptr::NonNull;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::ops::Deref;

use crate::hash::{DICT_CAN_RESIZE, DICT_FORCE_RESIZE_RATIO, DICT_HT_INITIAL_EXP, DICT_HT_INITIAL_SIZE, DictResizeFlag, HASHTABLE_MIN_FILL};
use crate::hash::error::HashError;
use crate::hash::hash::{sys_hash};

#[inline]
fn dict_size(exp: i32) -> u64 {
    return if exp == -1 {
        0
    } else {
        1 << exp
    }
}

#[inline]
fn dict_size_mask(exp: i32) -> u64 {
    return if exp == -1 {
        0
    } else {
        dict_size(exp) - 1
    }
}

#[derive(Debug)]
pub struct DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    key: K,
    val: V,
    next: Option<NonNull<DictEntry<K, V>>>,
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

    #[inline]
    pub fn push_back(&mut self, entry: DictEntry<K, V>) {
        unsafe {
            self.next = Some(NonNull::new_unchecked(Box::into_raw(Box::new(entry))));
        }
    }

    #[inline]
    pub fn dict_is_empty(&self) -> bool {
        self.key == K::default() && self.val == V::default()
    }

    #[inline]
    pub fn dict_get_key(&self) -> K {
        self.key.clone()
    }

    #[inline]
    pub fn dict_get_val(&self) -> V {
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
    pub fn dict_get_next(&self) -> Option<DictEntry<K, V>> {
        unsafe {
            self.next.map(|next| (*next.as_ptr()).clone())
        }
    }

    #[inline]
    pub fn set_next(&mut self, next: &mut DictEntry<K, V>) {
        unsafe {
            self.next = Some(NonNull::new_unchecked(next as *mut DictEntry<K, V>));
        }
    }
}

#[derive(Clone)]
pub struct Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    /// hash table
    ht_table: Vec<Vec<DictEntry<K, V>>>,
    /// hash table used
    ht_used: Vec<u32>,
    /// rehashing not in progress if rehash_idx == -1
    rehash_idx: i64,
    /// If >0 rehashing is paused
    pause_rehash: u64,
    /// exponent of size. (size = 1<<exp)
    ht_size_exp: Vec<i32>,
}

impl <K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub unsafe fn dict_add(&mut self, key: K, val: V) -> Result<(), HashError> {
        let hash = sys_hash(&key);
        let mut idx = hash & dict_size_mask(*self.ht_size_exp.get(0).unwrap());

        let mut tail = DictEntry::default();

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked(table));
            let head = self.ht_table.get_unchecked_mut(table).get_unchecked_mut(idx as usize);
            let mut tail = &DictEntry::default();
            loop {
                if *head == DictEntry::default() {
                    break;
                }
                let he_key = head.dict_get_key();
                if he_key == key {
                    return Err(HashError::DictEntryDup);
                }
                tail = head;
                if let Some(next) = head.dict_get_next() {
                    *head = next;
                } else {
                    break;
                }
            }
            if !self.dict_is_rehashing() { break; }
        }

        let ht_idx: usize = if self.dict_is_rehashing() { 1 } else { 0 };
        
        if tail == DictEntry::default() {
            *self.ht_table.get_unchecked_mut(ht_idx).get_unchecked_mut(idx as usize) = DictEntry::new(key, val);
        } else {
            tail.push_back(DictEntry::new(key, val));
        }
        *self.ht_used.get_mut(ht_idx).unwrap() += 1;

        Ok(())
    }

    pub unsafe fn dict_find(&mut self, key: K) -> Result<DictEntry<K, V>, HashError> {
        if self.dict_size() == 0 {
            return Err(HashError::DictEmpty);
        }

        let hash = sys_hash(key.clone());
        let idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked_mut(0));
        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            let idx = hash & dict_size_mask(*self.ht_size_exp.get_unchecked_mut(table));
            let head = self.ht_table.get_unchecked_mut(table).get_unchecked_mut(idx as usize);
            loop {
                if *head == DictEntry::default() {
                    break;
                }
                let he_key = head.dict_get_key();
                if he_key == key {
                    return Ok(head.clone());
                }
                if let Some(next) = head.dict_get_next() {
                    *head = next;
                } else {
                    break;
                }
            }
            if !self.dict_is_rehashing() { break; }
        }
        Err(HashError::DictNoKey)
    }

    pub unsafe fn dict_get_val(&mut self, key: &K) -> Option<V> {
        let hash = sys_hash(key.clone());
        let idx = hash & dict_size_mask(*self.ht_size_exp.get(0).unwrap());
        let table: usize = if self.dict_is_rehashing() { 1 } else { 0 };
        let head = self.ht_table.get_mut(table).unwrap().get_mut(idx as usize).unwrap();

        loop {
            if *head == DictEntry::default() {
                break;
            }
            let he_key = head.dict_get_key();
            if he_key == *key {
                let val = head.dict_get_val();
                return Some(val);
            }
            if let Some(next) = head.dict_get_next() {
                *head = next;
            } else {
                break;
            }
        }
        None
    }

    pub unsafe fn dict_delete(&mut self, key: K) -> Result<DictEntry<K, V>, HashError> {
        let hash = sys_hash(&key);
        let mut idx = hash & dict_size_mask(*self.ht_size_exp.get(0).unwrap());

        for table in 0..2 {
            if table == 0 && (idx as i64) < self.rehash_idx {
                continue;
            }
            idx = hash & dict_size_mask(*self.ht_size_exp.get(table).unwrap());
            let mut he = std::mem::replace(self.ht_table.get_unchecked_mut(table).get_unchecked_mut(idx as usize), DictEntry::default());
            let mut prev = DictEntry::default();
            loop {
                if he == DictEntry::default() {
                    break;
                }
                let he_key = he.dict_get_key();
                let next = he.dict_get_next();
                if he_key == key {
                    if let Some(mut next) = next {
                        if !prev.dict_is_empty() {
                            prev.set_next(&mut next);
                        } else {
                            *self.ht_table.get_unchecked_mut(table).get_unchecked_mut(idx as usize) = next;
                        }
                    }
                    *self.ht_used.get_unchecked_mut(table) -= 1;
                    return Ok(he);
                }
                prev = std::mem::replace(&mut he, DictEntry::default());
                if let Some(next) = next {
                    he = next;
                } else {
                    break;
                }
            }
        }
        Err(HashError::DictNoKey)
    }

    pub unsafe fn dict_get_table(&mut self) -> &Vec<DictEntry<K, V>> {
        self.ht_table.get_unchecked_mut(0)
    }

    pub unsafe fn rehash_at_index(&mut self, idx: usize) {
        let mut cur = std::mem::replace(self.ht_table.get_unchecked_mut(0).get_unchecked_mut(idx), DictEntry::default());
        loop {
            if cur == DictEntry::default() {
                break;
            }
            let next = cur.dict_get_next();
            let key = cur.dict_get_key();
            let mut idx = 0;
            if *self.ht_size_exp.get_unchecked(1) > *self.ht_size_exp.get_unchecked(0)
            {
                idx = sys_hash(&key) & dict_size_mask(*self.ht_size_exp.get_unchecked(1));
            }
            let mut target_entry = self.ht_table.get_unchecked_mut(1).get_unchecked_mut(idx as usize);
            cur.set_next(std::mem::replace(&mut target_entry, &mut DictEntry::default()));
            *self.ht_table.get_unchecked_mut(1).get_unchecked_mut(idx as usize) = cur;
            *self.ht_used.get_unchecked_mut(0) -= 1;
            *self.ht_used.get_unchecked_mut(1) += 1;
            if let Some(next_entry) = next {
                cur = next_entry;
            } else {
                break;
            }
        }
        *self.ht_table.get_unchecked_mut(0).get_unchecked_mut(idx) = DictEntry::default();
    }

    unsafe fn check_rehashing_complete(&mut self) -> bool {
        if *self.ht_used.get_unchecked(0) != 0 {
            return false;
        }
        *self.ht_table.get_unchecked_mut(0) = std::mem::replace(&mut *self.ht_table.get_unchecked_mut(1), vec![DictEntry::default(); 0]);
        *self.ht_used.get_unchecked_mut(0) = *self.ht_used.get_unchecked(1);
        *self.ht_size_exp.get_unchecked_mut(0) = *self.ht_size_exp.get_unchecked(1);
        self.reset(1);
        self.rehash_idx = -1;
        true
    }

    pub unsafe fn rehash(&mut self, mut n: usize) -> Result<bool, HashError> {
        let mut empty_visits = n * 10;
        if DICT_CAN_RESIZE == DictResizeFlag::DictResizeEnable || !self.dict_is_rehashing() {
            return Err(HashError::RehashErr);
        }
        let s0 = dict_size(*self.ht_size_exp.get(0).unwrap());
        let s1 = dict_size(*self.ht_size_exp.get(1).unwrap());
        if DICT_CAN_RESIZE == DictResizeFlag::DictResizeAvoid && ((s1 > s0 && s1 < DICT_FORCE_RESIZE_RATIO * s0) || (s1 < s0 && s0 < HASHTABLE_MIN_FILL * DICT_FORCE_RESIZE_RATIO * s1)) {
            return Err(HashError::RehashErr);
        }

        while n != 0 || *self.ht_used.get(0).unwrap() != 0 {
            assert!(dict_size(*self.ht_size_exp.get_unchecked(0)) as i64 > self.rehash_idx);
            while *self.ht_table.get_unchecked(0).get(self.rehash_idx as usize).unwrap() == DictEntry::default() {
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
        Ok(true)
    }
}

impl <K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn new() -> Self {
        Self {
            ht_table: vec![vec![DictEntry::default(); DICT_HT_INITIAL_SIZE]; 2],
            ht_used: vec![0; 2],
            rehash_idx: -1,
            pause_rehash: 0,
            ht_size_exp: vec![DICT_HT_INITIAL_EXP as i32; 2],
        }
    }

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
    fn dict_is_rehashing(&self) -> bool {
        self.rehash_idx != -1
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn dict_insert()  -> Result<(), HashError>{
        unsafe {
            let mut dict = Dict::new();
            dict.dict_add("hello", "world").unwrap();
            dict.dict_add("hello1", "world").unwrap();
            if let Ok(ret) = dict.dict_find("hello") {
                let v = ret.dict_get_val();
                println!("val: {}", v);
                let val = dict.dict_get_val(&"hello");
                assert_eq!("world", v);
                assert_eq!(Some("world"), val);
            }
            let res = dict.dict_delete("hello")?;
            println!("Dict delete: {:?}", res);
            match dict.dict_find("hello") {
                Ok(res) => {
                    return Ok(())
                }
                Err(e) => {
                    println!("Error：{}", e);
                }
            }
        }
        Ok(())
    }
}


use std::hash::Hash;
use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::dict::dict::{Dict, DictEntry};
use crate::dict::lib::{*};

#[derive(Debug)]
pub struct EntryIter<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    cur: Option<NonNull<DictEntry<K, V>>>,
    _boo: PhantomData<&'a (K, V)>,
}

impl<K, V> DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub unsafe fn iter(&self) -> EntryIter<K, V> {
        EntryIter {
            cur: Some(NonNull::new_unchecked(Box::into_raw(Box::new(DictEntry {
                key: self.key.clone(),
                val: self.val.clone(),
                next: self.next }
            )))),
            _boo: PhantomData,
        }
    }
}

impl<'a, K, V> Iterator for EntryIter<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    type Item = &'a DictEntry<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cur) = self.cur {
            unsafe {
                self.cur = (*cur.as_ptr()).next;
                Some(&(*cur.as_ptr()))
            }
        } else {
            None
        }
    }
}

pub struct DictIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    dict: &'a Dict<'a, K, V>,
    table: usize,
    index: i64,
    safe: i64,
    entry: Option<EntryIter<'a, K, V>>,
}

impl<K, V> Dict<'_, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn to_iter(&self) -> DictIterator<K, V> {
        unsafe {
            DictIterator {
            dict: self,
            table: 0,
            index: -1,
            safe: 0,
            entry: None,
        }
        }
    }
}

impl<'a, K, V> Iterator for DictIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    type Item = &'a DictEntry<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            loop {
                if let Some(ref mut iter) = self.entry {
                    if let Some(next) = iter.next() {
                        if *next != DictEntry::default() {
                            return Some(next);
                        }
                    }
                    self.entry = None;
                } else {
                    if self.table == 0 && self.index == -1 {
                        // if self.safe == 1 {
                        //     self.dict.pause_rehash();
                        //}
                        if self.dict.dict_is_rehashing() {
                            self.index = self.dict.get_rehash_idx() - 1;
                        }
                    }
                }
                self.index += 1;
                if self.index >= (dict_size(self.dict.ht_size_exp[self.table]) as i64) {
                    if self.dict.dict_is_rehashing() && self.table == 0 {
                        self.table += 1;
                        self.index = 0;
                    } else {
                        break;
                    }
                }
                let entry_iter = (*self.dict.ht_table[self.table][self.index as usize].unwrap().as_ptr()).iter();
                self.entry = Some(entry_iter);
            }
        }
        None
    }
}
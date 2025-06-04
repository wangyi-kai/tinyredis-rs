use std::hash::Hash;
use std::marker::PhantomData;
use crate::dict::dict::{Dict, DictEntry};
use crate::dict::lib::{*};

#[derive(Debug)]
pub struct EntryIter<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    cur: Option<&'a DictEntry<K, V>>,
    _boo: PhantomData<&'a (K, V)>,
}

impl<K, V> DictEntry<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn iter(&self) -> EntryIter<K, V> {
        EntryIter {
            cur: Some(self),
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
                let next = cur.next;
                if next.is_some() {
                    self.cur = Some(&(*next.unwrap().as_ptr()));
                } else {
                    self.cur = None;
                };
                Some(cur)
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
    pub(crate) dict: Option<&'a mut Dict<K, V>>,
    pub(crate) table: usize,
    pub(crate) index: i64,
    pub(crate) safe: i64,
    pub(crate) entry: Option<EntryIter<'a, K, V>>,
}

impl <'a, K, V> DictIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn reset(&mut self) {
        if !(self.index == -1 && self.table == 0)  {
            if self.safe != 0 {
                self.dict.as_mut().unwrap().resume_rehash()
            }
        }
    }
}

impl<'a, K, V> Dict<K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    pub fn iter(&mut self) -> DictIterator<K, V> {
        DictIterator {
            dict: Some(self),
            table: 0,
            index: -1,
            safe: 0,
            entry: None,
        }
    }

    pub fn safe_iter(&mut self) -> DictIterator<K, V> {
        DictIterator {
            dict: Some(self),
            table: 0,
            index: -1,
            safe: 1,
            entry: None,
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
                        if self.safe == 1 {
                            self.dict.as_mut().unwrap().pause_rehash();
                        }
                        if self.dict.as_ref().unwrap().dict_is_rehashing() {
                            self.index = self.dict.as_ref().unwrap().get_rehash_idx() - 1;
                        }
                    }
                }
                self.index += 1;
                if self.index >= (dict_size(self.dict.as_ref().unwrap().ht_size_exp[self.table]) as i64) {
                    if self.dict.as_ref().unwrap().dict_is_rehashing() && self.table == 0 {
                        self.table += 1;
                        self.index = 0;
                    } else {
                        break;
                    }
                }
                let entry_iter = (*self.dict.as_ref().unwrap().ht_table[self.table][self.index as usize].unwrap().as_ptr()).iter();
                self.entry = Some(entry_iter);
            }
        }
        None
    }
}
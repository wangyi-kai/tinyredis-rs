use std::hash::Hash;
use crate::kvstore::kvstore::KvStore;
use crate::dict::hash_iter::DictIterator;

pub struct KvStoreIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    kvs: KvStore<'a, K, V>,
    didx: usize,
    next_didx: usize,
    di: DictIterator<'a, K, V>,
}

pub struct KvStoreDictIterator<'a, K, V>
where K: Default + Clone + Eq + Hash,
      V: Default + PartialEq + Clone
{
    kvs: KvStore<'a, K, V>,
    didx: usize,
    di: DictIterator<'a, K, V>,
}


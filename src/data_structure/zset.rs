use crate::data_structure::dict::dict::Dict;
use crate::data_structure::dict::lib::DictType;
use crate::data_structure::skiplist::skiplist;
use crate::data_structure::skiplist::skiplist::SkipList;
use std::sync::Arc;

pub struct ZSet<K, V> {
    dict: Dict<K, V>,
    zsl: skiplist,
}

impl<K, V> ZSet<K, V> {
    pub fn new(dict_type: Arc<DictType<K, V>>) -> Self<K, V> {
        Self {
            dict: Dict::create(dict_type.clone()),
            zsl: SkipList::new(),
        }
    }
}

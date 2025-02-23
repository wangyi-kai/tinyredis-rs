use std::ptr::NonNull;

#[derive(Clone, Debug, Default)]
pub struct DictEntry<K, V> {
    key: K,
    val: V,
    next: Option<NonNull<DictEntry<K, V>>>,
}

impl <K, V> DictEntry<K, V> {
    pub fn new(key: K, val: V) -> Self {
        Self {
            key,
            val,
            next: None,
        }
    }
}

impl<K, V> Default for DictEntry<K, V> {
    fn default() -> Self {
        Self {
            key: None,
            val: None,
            next: None,
        }
    }
}

pub struct Dict<K, V> {
    /// hash table
    ht_table: Vec<DictEntry<K, V>>,
    /// hash table used
    ht_used: Vec<u32>,
    /// rehashing not in progress if rehash_idx == -1
    rehash_idx: i32,
}

impl <K, V> Dict<K, V> {
    #[inline]
    pub fn reset(&mut self, htidx: usize) {
        self.ht_table.get(htidx) = Some(&DictEntry::default());
        self.ht_used.get(htidx) = Some(&0);
        self.rehash_idx = -1;
    }

    #[inline]
    pub fn init(&mut self) -> Result<()> {
        self.reset(0);
        self.reset(1);
        Ok(())
    }
}



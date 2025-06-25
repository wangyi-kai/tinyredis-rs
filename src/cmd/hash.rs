use crate::data_structure::dict::dict::Value;
use crate::db::db::RedisDb;
use crate::server::{OBJ_ENCODING_HT, RedisObject, RedisValue};

pub enum HashCmd {
    /// Creates or modifies the value of a field in a hash
    HSet { key: String, value: String},
    /// Returns the value of a field in a hash
    HGet { key: String },
    /// Deletes one or more fields and their values from a hash.
    HDel { key: String },
    /// Iterates over fields and values of a hash
    HScan,
}

impl HashCmd {
    pub fn set(&self, db: &mut RedisDb<RedisObject<String>>, key: RedisObject<String>) {
        let mut o = db.lookup_key(&key);
        if o.is_none() {
            let mut ht = RedisObject::create_hash_object();
            match self {
                HashCmd::HSet {key, value} => Self::hash_set(&mut ht, key.clone(), value.clone()),
                _ => return
            }
            db.add(key, ht);
        }

        match self {
            HashCmd::HSet {key, value} => {
                Self::hash_set(o.unwrap(), key.clone(), value.clone());
            }
            _ => return
        }
    }


    fn hash_set(o: &mut RedisObject<String>, field: String, value: String) {
        if o.encoding == OBJ_ENCODING_HT {
            let mut ht = match &mut o.ptr {
                RedisValue::Hash(ht) => ht,
                _ => return,
            };
            let entry = ht.find(&field);
            unsafe {
                if entry.is_some() {
                    (*entry.unwrap().as_ptr()).val = value;
                } else {
                    ht.add_raw(field, value).ok();
                }
            }
        }
    }

    pub fn get(&self, db: &mut RedisDb<RedisObject<String>>, key: &RedisObject<String>, field: &str) -> Option<&str> {
        let mut o = db.lookup_key(&key);
        if o.is_none() {
            return None;
        }
        let val = Self::get_value(o.unwrap(), field);
        Some(val)
    }

    fn get_value(o: &mut RedisObject<String>, field: &str) -> &'static str {
        if o.encoding == OBJ_ENCODING_HT {
            let de = match &mut o.ptr {
                RedisValue::Hash(ht) => ht.find(&field),
                _ => return &"".to_string()
            };
            unsafe {
                let value = &(*de.unwrap().as_ptr()).val;
                value
            }
        } else {
            &"".to_string()
        }
    }

    pub fn delete(&self, db: &mut RedisDb<RedisObject<String>>, key: &RedisObject<String>) {
        let mut o = db.lookup_key(&key);
        if o.is_none() {
            return;
        }
        match self {
            HashCmd::HDel { key} => {
                Self::hash_delete(o.unwrap(), &key);
            }
            _ => {}
        }
    }

    fn hash_delete(o: &mut RedisObject<String>, field: &str) -> bool {
        let mut deleted = false;
        if o.encoding == OBJ_ENCODING_HT {
            return match &mut o.ptr {
                RedisValue::Hash(ht) => {
                    ht.generic_delete(field).ok();
                    true
                }
                _ => deleted,
            }
        } else {
            deleted
        }
    }
}
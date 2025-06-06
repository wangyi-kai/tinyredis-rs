use crate::cluster::cluster::key_hash_slot;

pub fn get_key_slot(key: &str) -> i32 {
    key_hash_slot(key) as i32
}
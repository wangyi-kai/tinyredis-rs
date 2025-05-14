use std::hash::{Hash, Hasher};
use cityhasher::CityHasher;

pub const HASH_SEED: u64 = 0x87c37b91114253d5;

#[inline]
pub fn sys_hash(hash: impl Hash) -> u64 {
    let mut hasher = CityHasher::with_seed(HASH_SEED);
    hash.hash(&mut hasher);
    hasher.finish()
}

#[inline]
pub fn sys_hash_16(hash: impl Hash) -> u16 {
    let hash = sys_hash(hash);
    let hash = hash ^ (hash >> 32);
    (hash & 0xFFFF) as u16
}
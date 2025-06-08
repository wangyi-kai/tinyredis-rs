use std::cmp::Ordering;
use rand::Rng;
use super::{RAND_MAX, SKIP_LIST_MAX_LEVEL, SKIP_LIST_P};

pub fn gen_random() -> u32 {
    let mut rng = rand::rng();
    rng.random::<u32>()
}

pub fn random_level() -> usize {
    let threshold = SKIP_LIST_P * RAND_MAX as f32;
    let mut level = 1;
    while (gen_random() as f32) < threshold {
        level += 1;
    }
    level.min(SKIP_LIST_MAX_LEVEL)
}

pub fn sds_cmp(s1: &str, s2: &str) -> i32 {
    let l1 = s1.len();
    let l2 = s2.len();
    let min_len = if l1 < l2 { l1 } else { l2 };
    let order = s1[..min_len].cmp(&s2[..min_len]);
    match order {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[cfg(test)]
mod dict_test {
    use crate::dict::dict::Dict;
    use crate::dict::error::HashError;

    use std::collections::HashMap;
    use std::time::{Instant};
    use rand::{Rng, SeedableRng};
    use rand::rngs::StdRng;

    use std::fmt::Write as _;
    use std::sync::Arc;
    use crate::dict::hash::sys_hash;
    use crate::dict::lib::{DictResizeFlag::DictResizeAvoid, dict_set_resize_enabled, DICT_FORCE_RESIZE_RATIO, next_exp, dict_size, HASHTABLE_MIN_FILL, random_u32, random_i32, DictType};
    use crate::dict::lib::DictResizeFlag::DictResizeEnable;

    fn string_from_long_long(value: i64) -> String {
        let mut s = String::with_capacity(32);
        let _ = write!(&mut s, "{}", value);
        s
    }

    fn string_from_substring() -> String {
        const LARGE_STRING_SIZE: usize = 1000;
        const MIN_STRING_SIZE: usize = 100;
        const MAX_STRING_SIZE: usize = 500;

        let mut large_string = String::with_capacity(LARGE_STRING_SIZE + 1);
        let mut init = 0;
        if init == 0 {
            // Generate a large string
            for i in 0..LARGE_STRING_SIZE {
                large_string.push((33 + rand::rng().random::<u32>() as u8 % 94) as char);
            }
            init = 1;
        }

        let sub_string_size = MIN_STRING_SIZE + rand::rng().random::<u32>() as usize % (MAX_STRING_SIZE - MIN_STRING_SIZE + 1);
        let start_index = rand::rng().random::<u32>() as usize % (LARGE_STRING_SIZE - sub_string_size + 1);
        large_string[start_index..start_index + sub_string_size].to_string()
    }

    #[test]
    fn insert_test() -> Result<(), HashError> {
        let benchmark_dict_type = Arc::new(DictType {
            hash_function: None,
            rehashing_started: None,
            rehashing_completed: None,
            dict_meta_data_bytes: None,
        });
        let mut d = Dict::create(benchmark_dict_type);
        let mut current_dict_used = 0;
        let mut new_dict_size = 0;

        print!("[TEST] Add 16 keys and verify dict resize is ok: ");
        {
            for j in 0..16 {
                unsafe {
                    d.add_raw(string_from_long_long(j), j)?;
                }
            }
            while d.dict_is_rehashing() {
                d.rehash_microseconds(1000)?;
            }
            assert_eq!(d.dict_size(), 16);
            assert_eq!(d.dict_buckets(), 16);
            println!("PASS");
        }

        print!("[TEST] Use DICT_RESIZE_AVOID to disable the dict resize and pad to (dict_force_resize_ratio * 16): ");
        {
            dict_set_resize_enabled(DictResizeAvoid);
            for j in 16..DICT_FORCE_RESIZE_RATIO as i64 * 16 {
                unsafe {
                    let res = d.add_raw(string_from_long_long(j), j)?;
                    //assert_eq!(res, true);
                }
            }
            current_dict_used = DICT_FORCE_RESIZE_RATIO * 16;
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(d.dict_buckets(), 16);
            println!("PASS");
        }

        print!("[TEST] Add one more key, trigger the dict resize: ");
        unsafe {
            let res = d.add_raw(string_from_long_long(current_dict_used as i64), current_dict_used as i64)?;
            //assert_eq!(res, true);
            current_dict_used += 1;
            new_dict_size = 1 << next_exp(current_dict_used as usize);
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(dict_size(d.ht_size_exp[0]), 16);
            assert_eq!(dict_size(d.ht_size_exp[1]), new_dict_size);

            // Wait for rehashing
            dict_set_resize_enabled(DictResizeEnable);
            while d.dict_is_rehashing() {
                d.rehash_microseconds(1000)?;
            }
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(dict_size(d.ht_size_exp[0]), new_dict_size);
            assert_eq!(dict_size(d.ht_size_exp[1]), 0);
            println!("PASS");
        }

        print!("[TEST] Delete keys until we can trigger shrink in next test: ");
        // Delete keys until we can satisfy (1 / HASHTABLE_MIN_FILL) in the next test.
        unsafe {
            for j in (new_dict_size / HASHTABLE_MIN_FILL + 1)..current_dict_used {
                let key = string_from_long_long(j as i64);
                let res = d.generic_delete(&key);
            }
            current_dict_used = new_dict_size / HASHTABLE_MIN_FILL + 1;
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(dict_size(d.ht_size_exp[0]), new_dict_size);
            assert_eq!(dict_size(d.ht_size_exp[1]), 0);
            println!("PASS");
        }

        print!("[TEST] Delete one more key, trigger the dict resize: ");
        {
            current_dict_used -= 1;
            let key = string_from_long_long(current_dict_used as i64);
            unsafe {
                d.generic_delete(&key)?;
            }
            let old_dict_size = new_dict_size;
            new_dict_size = 1 << next_exp(current_dict_used as usize);
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(dict_size(d.ht_size_exp[0]), old_dict_size);
            assert_eq!(dict_size(d.ht_size_exp[1]), new_dict_size);

            // Wait for rehashing
            while d.dict_is_rehashing() {
                d.rehash_microseconds(1000)?;
            }
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(dict_size(d.ht_size_exp[0]), new_dict_size);
            assert_eq!(dict_size(d.ht_size_exp[1]), 0);
            println!("PASS");
        }

        print!("[TEST] Empty the dictionary and add 128 keys: ");
        {
            d.empty(None);
            for j in 0..128 {
                d.add_raw(string_from_long_long(j), j)?;
            }
            while d.dict_is_rehashing() {
                d.rehash_microseconds(1000)?;
            }
            assert_eq!(d.dict_size(), 128);
            assert_eq!(d.dict_buckets(), 128);
            println!("PASS");
        }

        print!("[TEST] Use DICT_RESIZE_AVOID to disable the dict resize and reduce to 3: ");
        {
            dict_set_resize_enabled(DictResizeAvoid);
            let remain_keys = dict_size(d.ht_size_exp[0]) / (HASHTABLE_MIN_FILL * DICT_FORCE_RESIZE_RATIO) + 1;
            for j in remain_keys..128 {
                let key = string_from_long_long(j as i64);
                let res = d.generic_delete(&key);
            }
            current_dict_used = remain_keys;
            assert_eq!(d.dict_size() as u64, remain_keys);
            assert_eq!(d.dict_buckets(), 128);
            println!("PASS");
        }

        print!("[TEST] Delete one more key, trigger the dict resize: ");
        {
            current_dict_used -= 1;
            let key = string_from_long_long(current_dict_used as i64);
            let res = d.generic_delete(&key)?;
            new_dict_size = 1 << next_exp(current_dict_used as usize);
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(dict_size(d.ht_size_exp[0]), 128);
            assert_eq!(dict_size(d.ht_size_exp[1]), new_dict_size);

            dict_set_resize_enabled(DictResizeEnable);
            while d.dict_is_rehashing() {
                d.rehash_microseconds(1000)?;
            }
            assert_eq!(d.dict_size() as u64, current_dict_used);
            assert_eq!(dict_size(d.ht_size_exp[0]), new_dict_size);
            assert_eq!(dict_size(d.ht_size_exp[1]), 0);
            println!("PASS");
        }
        Ok(())
    }
    #[test]
    fn restore() -> Result<(), HashError> {
        let benchmark_dict_type = Arc::new(DictType {
            hash_function: None,
            rehashing_started: None,
            rehashing_completed: None,
            dict_meta_data_bytes: None,
        });
        let mut d = Dict::create(benchmark_dict_type);
        println!("[TEST] Restore to original state: ");
        {
            d.empty(None);
            dict_set_resize_enabled(DictResizeEnable);
        }
        let rng = StdRng::seed_from_u64(12345);
        let start = Instant::now();
        let count = 5000;
        for j in 0..count {
            let key = string_from_substring();
            d.add_raw(key, 0)?;
        }
        let end = start.elapsed();
        println!("Inserting random substrings (100-500B) from large string with symbols: {:?}", end);
        assert!(d.dict_size() <= count);
        d.empty(None);

        let start = Instant::now();
        for j in 0..count {
            let key = string_from_long_long(j as i64);
            d.add_raw(key, j)?;
        }
        let end = start.elapsed();
        println!("Inserting via dictAdd() non existing: {:?}", end);
        assert_eq!(d.dict_size(), count);
        d.empty(None);

        let start = Instant::now();
        for j in 0..count {
            let key = string_from_long_long(j as i64);
            let hash = sys_hash(&key);
            d.add_non_exists_by_hash(key, hash);
        }
        let end = start.elapsed();
        println!("Inserting via dictAddNonExistsByHash() non existing: {:?}", end);
        assert_eq!(d.dict_size(), count);

        while d.dict_is_rehashing() {
            d.rehash_microseconds(100 * 1000)?;
        }
        d.empty(None);

        let start = Instant::now();
        unsafe {
            for j in 0..count {
                let key = string_from_long_long(j as i64);
                let entry = d.find(&key);
                assert!(entry.is_none());

                let res = d.add_raw(key, 0)?;
                //assert_eq!(res, true);
            }
        }
        let end = start.elapsed();
        println!("Find() and inserting via dictFind()+dictAddRaw() non existing: {:?}", end);
        d.empty(None);

        let start = Instant::now();
        for j in 0..count {
            let key = string_from_long_long(j as i64);
            let hash = sys_hash(&key);
            let entry = d.find_by_hash(&key, hash);
            assert!(entry.is_none());
            d.add_non_exists_by_hash(key, hash);
        }
        let end = start.elapsed();
        println!("Find() and inserting via dictGetHash()+dictFindByHash()+dictAddNonExistsByHash() non existing: {:?}", end);
        assert_eq!(d.dict_size(), count);

        while d.dict_is_rehashing() {
            d.rehash_microseconds(100 * 1000)?;
        }

        let start = Instant::now();
        for j in 0..count {
            let key = string_from_long_long(j as i64);
            let entry = d.find(&key);
            assert!(entry.is_some());
        }
        let end = start.elapsed();
        println!("Linear access of existing elements: {:?}", end);

        let start = Instant::now();
        for j in 0..count {
            let key = string_from_long_long((random_u32() % count) as i64);
            let entry = d.find(&key);
            assert!(entry.is_some());
        }
        let end = start.elapsed();
        println!("Random access of existing elements: {:?}", end);

        let start = Instant::now();
        for j in 0..count {
            let de = d.get_random_key();
            assert!(de.is_some());
        }
        let end = start.elapsed();
        println!("Accessing random keys: {:?}", end);

        let start = Instant::now();
        for j in 0..count {
            let mut key = string_from_long_long((random_u32() % count) as i64);
            key.replace_range(0..1, "X");
            let de = d.find(&key);
            assert!(de.is_none());
        }
        let end = start.elapsed();
        println!("Accessing missing: {:?}", end);

        let start = Instant::now();
        for j in 0..count {
            let mut key = string_from_long_long(j as i64);
            d.generic_delete(&key)?;
            let c = key.chars().nth(0).unwrap() as u8;
            key.replace_range(0..1, &(c + 17).to_string());
            d.add_raw(key, j)?;
        }
        let end = start.elapsed();
        println!("Removing and adding: {:?}", end);

        Ok(())
    }


    #[test]
    fn dict_insert_and_find()  -> Result<(), HashError>{
        unsafe {
            let benchmark_dict_type = Arc::new(DictType {
                hash_function: None,
                rehashing_started: None,
                rehashing_completed: None,
                dict_meta_data_bytes: None,
            });
            let mut dict = Dict::create(benchmark_dict_type);
            let num = 10;
            let start = Instant::now();

            for i in 1..num + 1 {
                let key = format!("{}", i.to_string());
                let value = format!("val_{}", i.to_string());
                let _ = dict.add_raw(key, value);
            }
            let end = start.elapsed();
            println!("dict插入时间: {:?}", end);

            let mut ht = HashMap::new();
            let st = Instant::now();
            for i in 1..num + 1 {
                let key = format!("{}", i.to_string());
                let value = format!("val_{}", i.to_string());
                ht.insert(key, value);
            }
            let iter = ht.iter();
            let ed = st.elapsed();
            println!("hashmap插入时间: {:?}", ed);

            for i in 1..num + 1 {
                let key = format!("{}", i.to_string());
                let entry = dict.find(&key);
                match entry {
                    Some(entry) => {
                        let val = (*entry.as_ptr()).get_val();
                        assert_eq!(format!("val_{}", i.to_string()), *val);
                        //println!("找到要查找key: {}, entry: {:?}", key, val);
                    }
                    None => {
                        println!("没有找到key: {}", key);
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn dict_iter() -> Result<(), HashError> {
        unsafe {
            let benchmark_dict_type = Arc::new(DictType {
                hash_function: None,
                rehashing_started: None,
                rehashing_completed: None,
                dict_meta_data_bytes: None,
            });
            let mut dict = Dict::create(benchmark_dict_type);
            let num = 10;
            let start = Instant::now();

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let value = format!("val_{}", i.to_string());
                let _ = dict.add_raw(key, value);
            }
            let end = start.elapsed();

            let mut count = 0;
            let iter = dict.iter();
            for entry in iter {
                println!("key: {}, val: {}", entry.get_key(), entry.get_val());
                count += 1;
            }
            assert_eq!(num, count);
        }
        Ok(())
    }


    #[test]
    fn dict_delete() -> Result<(), HashError> {
        unsafe {
            let benchmark_dict_type = Arc::new(DictType {
                hash_function: None,
                rehashing_started: None,
                rehashing_completed: None,
                dict_meta_data_bytes: None,
            });
            let mut dict = Dict::create(benchmark_dict_type);
            let num = 10;

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let value = format!("val_{}", i.to_string());
                let _ = dict.add_raw(key, value);
            }

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let _ = dict.generic_delete(&key);
            }

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let entry = dict.find(&key);
                match entry {
                    Some(entry) => {
                        let val = (*entry.as_ptr()).get_val();
                        assert_eq!(format!("val_{}", i.to_string()), *val);
                        //println!("找到要查找key: {}, entry: {:?}", key, val);
                    }
                    None => {
                        println!("没有找到key: {}", key);
                    }
                }
                //assert_eq!(None, entry);
            }
        }
        Ok(())
    }
}
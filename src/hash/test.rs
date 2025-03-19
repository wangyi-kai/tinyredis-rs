use crate::hash::dict::Dict;
use crate::hash::error::HashError;

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use std::fmt::format;
    use std::time::{Duration, Instant};
    use crate::hash::dict::DictEntry;

    #[test]
    fn dict_insert_and_find()  -> Result<(), HashError>{
        unsafe {
            let mut dict = Dict::new();
            let num = 10;
            let start = Instant::now();

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let value = format!("val_{}", i.to_string());
                let _ = dict.dict_add(key, value)?;
            }
            let end = start.elapsed();
            println!("dict插入时间: {:?}", end);

            // let mut ht = HashMap::new();
            // let st = Instant::now();
            // for i in 1..num + 1 {
            //     let key = format!("key_{}", i.to_string());
            //     let value = format!("val_{}", i.to_string());
            //     ht.insert(key, value);
            // }
            // let iter = ht.iter();
            // let ed = st.elapsed();
            // println!("hashmap插入时间: {:?}", ed);

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let entry = dict.dict_find(&key);
                match entry {
                    Some(entry) => {
                        let val = entry.get_val();
                        assert_eq!(format!("val_{}", i.to_string()), val);
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
            let mut dict = Dict::new();
            let num = 10;
            let start = Instant::now();

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let value = format!("val_{}", i.to_string());
                let _ = dict.dict_add(key, value)?;
            }
            let end = start.elapsed();

            let mut count = 0;
            let iter = dict.iter();
            for entry in iter {
                count += 1;
            }
            assert_eq!(num, count);
        }
        Ok(())
    }

    #[test]
    fn dict_delete() -> Result<(), HashError> {
        unsafe {
            let mut dict = Dict::new();
            let num = 10;

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let value = format!("val_{}", i.to_string());
                let _ = dict.dict_add(key, value)?;
            }

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let _ = dict.dict_delete(key)?;
            }

            for i in 1..num + 1 {
                let key = format!("key_{}", i.to_string());
                let entry = dict.dict_find(&key);
                assert_eq!(None, entry);
            }
        }
        Ok(())
    }
}
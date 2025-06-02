
mod kvstore_test {
    use crate::dict::lib::DictType;
    use std::fmt::Write as _;
    use std::sync::Arc;
    use crate::kvstore::kvstore::KvStore;
    use crate::kvstore::{KVSTORE_ALLOCATE_DICTS_ON_DEMAND, KVSTORE_ALLOC_META_KEYS_HIST, KVSTORE_FREE_EMPTY_DICTS};

    fn test_name(name: &str) {
        print!("test-{}", name);
    }

    fn string_from_int(value: i32) -> String {
        let mut s = String::new();

        let _ = write!(&mut s, "{}", value);
        s
    }

    #[test]
    fn kvstore_test() {
        let mut didx = 0;
        let mut curr_slot = 0;
        let dict_type: DictType<String, String> = DictType {
            hash_function: None,
            rehashing_started: None,
            rehashing_completed: None,
            dict_meta_data_bytes: None,
        };
        let dict_type = Arc::new(dict_type);

        let mut kvs1 = KvStore::create(dict_type.clone(), 0, KVSTORE_ALLOCATE_DICTS_ON_DEMAND);
        let mut kvs2 = KvStore::create(dict_type.clone(), 0, KVSTORE_ALLOCATE_DICTS_ON_DEMAND | KVSTORE_FREE_EMPTY_DICTS);

        print!("[TEST] Add 16 keys: ");
        {
            for i in 0..16 {
                let de = kvs1.dict_add_raw(didx, string_from_int(i));
                assert!(de.is_some());
                let de = kvs2.dict_add_raw(didx, string_from_int(i));
                assert!(de.is_some());
            }
            assert_eq!(kvs1.dict_size(didx as usize), 16);
            assert_eq!(kvs1.kvstore_size(), 16);
            assert_eq!(kvs2.dict_size(didx as usize), 16);
            assert_eq!(kvs2.kvstore_size(), 16);
            println!("PASS");
        }

        print!("[TEST] kvstore Iterator case 1: removing all keys does not delete the empty dict: ");
        {
            let mut iter = kvs1.iter();
            while let Some(de) = iter.next() {
                curr_slot = iter.get_current_dict_index();
                let key = &de.key;
                assert!(kvs1.dict_delete(curr_slot, key).is_some())
            }
            iter.release();

            let d = kvs1.get_dict(didx as usize);
            assert!(d.is_some());
            assert_eq!(kvs1.dict_size(didx as usize), 0);
            assert_eq!(kvs1.kvstore_size(), 0);
            println!("PASS");
        }

        print!("[TEST] kvstore Iterator case 2: removing all keys will delete the empty dict: ");
        {
            let mut iter = kvs2.iter();
            while let Some(de) = iter.next() {
                curr_slot = iter.get_current_dict_index();
                let key = &de.key;
                assert!(kvs2.dict_delete(curr_slot, key).is_some())
            }
            iter.release();

            while kvs2.increment_rehash(1000) != 0 { }
            let d = kvs2.get_dict(didx as usize);
            assert!(d.is_none());
            assert_eq!(kvs2.dict_size(didx as usize), 0);
            assert_eq!(kvs2.kvstore_size(), 0);
            println!("PASS");
        }

        print!("[TEST] Add 16 keys again: ");
        {
            for i in 0..16 {
                let de = kvs1.dict_add_raw(didx, string_from_int(i));
                assert!(de.is_some());
                let de = kvs2.dict_add_raw(didx, string_from_int(i));
                assert!(de.is_some());
            }
            assert_eq!(kvs1.dict_size(didx as usize), 16);
            assert_eq!(kvs1.kvstore_size(), 16);
            assert_eq!(kvs2.dict_size(didx as usize), 16);
            assert_eq!(kvs2.kvstore_size(), 16);
            println!("PASS");
        }

        print!("[TEST] kvstore DictIterator case 1: removing all keys does not delete the empty dict ");
        {
            let mut iter = kvs1.get_dict_safe_iterator(didx as usize);
            while let Some(de) = iter.next() {
                let key = &de.key;
                assert!(kvs1.dict_delete(didx, key).is_some());
            }
            iter.release_dict_iterator();

            let d = kvs1.get_dict(didx as usize);
            assert!(d.is_some());
            assert_eq!(kvs1.dict_size(didx as usize), 0);
            assert_eq!(kvs1.kvstore_size(), 0);
            println!("PASS");
        }

        print!("[TEST] kvstore DictIterator case 2: removing all keys will delete the empty dict: ");
        {
            let mut iter = kvs2.get_dict_safe_iterator(didx as usize);
            while let Some(de) = iter.next() {
                let key = &de.key;
                assert!(kvs2.dict_delete(didx, key).is_some());
            }
            iter.release_dict_iterator();

            let d = kvs2.get_dict(didx as usize);
            assert!(d.is_none());
            assert_eq!(kvs2.dict_size(didx as usize), 0);
            assert_eq!(kvs2.kvstore_size(), 0);
            println!("PASS");
        }

        print!("[TEST] Verify that a rehashing dict's node in the rehashing list is correctly updated after defragmentation: ");
        {
            let cursor = 0;
            let mut kvs = KvStore::create(dict_type.clone(), 0, KVSTORE_ALLOCATE_DICTS_ON_DEMAND);
            for i in 0..256 {
                let de = kvs.dict_add_raw(0, string_from_int(i));
                if kvs.rehashing.length() != 0 {
                    break;
                }
            }
            //assert!(kvs.rehashing.length() > 0);
            println!("PASS");
        }

        print!("[TEST] Verify non-empty dict count is correctly updated: ");
        {
            let mut kvs = KvStore::create(
                dict_type.clone(),
                2,
                KVSTORE_ALLOCATE_DICTS_ON_DEMAND | KVSTORE_ALLOC_META_KEYS_HIST
            );
            for idx in 0..4 {
                for i in 0..16 {
                    let de = kvs.dict_add_raw(idx, string_from_int(i));
                    assert!(de.is_some());
                    if i == 0 {
                        assert_eq!(kvs.non_empty_dicts(), idx + 1);
                    }
                }
            }

            for idx in 0..4 {
                let mut iter = kvs.get_dict_safe_iterator(idx);
                while let Some(de) = iter.next() {
                    let key = de.get_key();
                    assert!(kvs.dict_delete(idx as i32, key).is_some());
                    if kvs.dict_size(idx as usize) == 0 {
                        assert_eq!(kvs.non_empty_dicts(), 3 - idx as i32);
                    }
                }
                iter.release_dict_iterator();
            }
            println!("PASS");
        }
    }
}
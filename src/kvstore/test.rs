
mod kvstore_test {
    use crate::dict::lib::DictType;
    use std::fmt::Write as _;
    use std::sync::Arc;
    use crate::kvstore::kvstore::KvStore;
    use crate::kvstore::KVSTORE_ALLOCATE_DICTS_ON_DEMAND;

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
        let mut kvs2 = KvStore::create(dict_type.clone(), 0, KVSTORE_ALLOCATE_DICTS_ON_DEMAND);

        print!("[TEST] Add 16 keys");
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
        }
    }
}
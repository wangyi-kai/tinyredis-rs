
mod intset_test {
    use crate::intset::intset::IntSet;
    use crate::skiplist::lib::gen_random;
    use crate::intset::{INTSET_ENC_INT16, INTSET_ENC_INT32, INTSET_ENC_INT64};
    use crate::intset::lib::intset_value_encoding;

    fn intset_repr(is: &IntSet) {
        for i in 0..is.get_length() {
            print!("{} ", is.get(i as usize).unwrap());
        }
        print!("\n");
    }

    fn create_set(bits: i32, size: i32) -> IntSet {
        let mask: u64 = 1 << bits - 1;
        let mut is = IntSet::new();
        let mut value = 0;

        for i in 0.. size {
            if bits > 32 {
                value = (gen_random() as u64 * gen_random() as u64) & mask;
            } else {
                value = gen_random() as u64 & mask;
            }
            is.add(value as i64);
        }
        is
    }

    fn check_consistency(is: &IntSet) {
        for i in 0..is.get_length() as usize - 1 {
            let encoding = is.encoding as u8;

            match encoding {
                INTSET_ENC_INT16 => {
                    assert_eq!(is.contents.len() % 2, 0, "unaligned");
                    let slice = unsafe {
                        std::slice::from_raw_parts(is.contents.as_ptr() as *const i16, is.contents.len() / 2)
                    };
                    assert!(slice[i] < slice[i + 1]);
                }
                INTSET_ENC_INT32 => {
                    assert_eq!(is.contents.len() % 4, 0, "unaligned");
                    let slice = unsafe {
                        std::slice::from_raw_parts(is.contents.as_ptr() as *const i32, is.contents.len() / 4)
                    };
                    assert!(slice[i] < slice[i + 1]);
                }
                INTSET_ENC_INT64 => {
                    assert_eq!(is.contents.len() % 8, 0, "unaligned");
                    let slice = unsafe {
                        std::slice::from_raw_parts(is.contents.as_ptr() as *const i32, is.contents.len() / 8)
                    };
                    assert!(slice[i] < slice[i + 1]);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test() {
        print!("[TEST] Value encodings: ");
        {
            assert_eq!(intset_value_encoding(-32768), INTSET_ENC_INT16);
            assert_eq!(intset_value_encoding(32767), INTSET_ENC_INT16);
            assert_eq!(intset_value_encoding(-32769), INTSET_ENC_INT32);
            assert_eq!(intset_value_encoding(32768), INTSET_ENC_INT32);
            assert_eq!(intset_value_encoding(-2147483648), INTSET_ENC_INT32);
            assert_eq!(intset_value_encoding(2147483647), INTSET_ENC_INT32);
            assert_eq!(intset_value_encoding(-2147483649), INTSET_ENC_INT64);
            assert_eq!(intset_value_encoding(2147483648), INTSET_ENC_INT64);
            assert_eq!(intset_value_encoding(-9223372036854775808),
                    INTSET_ENC_INT64);
            assert_eq!(intset_value_encoding(9223372036854775807),
                    INTSET_ENC_INT64);
            println!("PASS");
        }

        print!("[TEST] Basic adding: ");
        {
            let mut is = IntSet::new();
            is.add(5);
            is.add(6);
            is.add(4);
            is.add(4);
            assert_eq!(6, is.get_max());
            assert_eq!(4, is.get_min());
            println!("PASS");
        }

        print!("[TEST] Large number of random adds: ");
        {
            let mut is = IntSet::new();
            let mut inserts = 0;
            for i in 0..1024 {
                is.add((gen_random() % 0x800) as i64);
                inserts += 1;
            }
            //assert_eq!(is.get_length(), inserts);
            check_consistency(&is);
            println!("PASS");
        }
    }
}
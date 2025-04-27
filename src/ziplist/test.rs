
#[cfg(test)]
mod test {
    use crate::ziplist::error::ZipListError;
    use crate::ziplist::ziplist::ZipList;
    use crate::ziplist::{ZIPLIST_HEAD, ZIPLIST_HEADER_SIZE, ZIPLIST_TAIL};

    use std::time::{SystemTime, UNIX_EPOCH, Instant};
    use rand::{Rng, rng};
    use rand::distr::Alphanumeric;
    use crate::ziplist::lib::ziplist_repr;
    use ansi_term::Color::{Green, Red};
    use std::slice;

    fn create() -> ZipList {
        let mut zl = ZipList::new();
        let _ = zl.push("foo", false);
        let _ = zl.push("quux", false);
        let _ = zl.push("hello", true);
        let _ = zl.push("1024", true);
        zl
    }

    fn create_int_list() -> ZipList {
        let mut zl = ZipList::new();
        let _ = zl.push("100", false);
        let _ = zl.push("12800", false);
        let _ = zl.push("-100", true);
        let _ = zl.push("4294967296", true);
        let _ = zl.push("non integer", false);
        let _ = zl.push("much much longer non integer", false);
        zl
    }

    fn usec() -> i64 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        (now.as_secs() as i64) * 1_000_000 + (now.subsec_micros() as i64)
    }

    fn stress(pos: usize, num: usize, max_size: usize, dnum: usize) {
        for i in (0..max_size).step_by(dnum) {
            let mut zl = ZipList::new();

            for j in 0..i {
                let _ = zl.push("quux", false);
            }

            let start = Instant::now();
            for k in 0..num {
                let _ = zl.push("quux", false);
                let _ = zl.delete_range(0, 1);
            }
            let end = start.elapsed();
            println!("List size: {i}, bytes: {}, push+pop: {}, time: {:?}", zl.ziplist_len(), num, end);
        }
    }

    fn pop(zl: &mut ZipList, where_: i32) {
        print!("[TEST]");
        let mut p = if where_ == ZIPLIST_HEAD {
            zl.zip_index(0)
        } else {
            zl.zip_index(-1)
        };
        let mut vlen: u32 = 0;
        let mut vstr = String::default();
        let mut vlong: i64 = 0;
        if zl.get(p, &mut vstr, &mut vlen, &mut vlong) {
            if where_ == ZIPLIST_HEAD {
                print!("Pop head: ");
            } else {
                print!("Pop tail ");
            }
            if !vstr.is_empty() {
                println!("{}", vstr);
            } else {
                println!("{}", vlong);
            }
            let _ = zl.delete(&mut p);
            return;
        } else {
            panic!("ERROR: Could not pop")
        }
    }

    fn rand_string(min_num: u32, max_num: u32) -> String {
        let rand: u32 = rand::rng().random();
        let len = min_num + rand % (max_num - min_num + 1);
        let mut s = String::default();
        let (min_val, max_val) = match rand::rng().random_range(0..3) {
            0 => (0, 255),
            1 => (48, 122),
            2 => (48, 52),
            _ => (0, 0),
        };
        for i in 0..len as usize {
            let num: i32 = rand::rng().random();
            let v = min_val + num % (max_val - min_val + 1);
            s += &*v.to_string();
        }
        s
    }

    fn verify(zl: ZipList) {
        let len = zl.ziplist_len();

    }

    #[test]
    fn ziplist_test() {
        let mut pos = 0;
        let mut entry = String::default();
        let mut elen: u32 = 0;
        let mut value: i64 = 0;
        let iteration: i32 = 0;

        let mut zl = create_int_list();
        ziplist_repr(&zl);

        zl = create();
        ziplist_repr(&zl);

        pop(&mut zl, ZIPLIST_TAIL);
        ziplist_repr(&zl);

        pop(&mut zl, ZIPLIST_HEAD);
        ziplist_repr(&zl);

        pop(&mut zl, ZIPLIST_TAIL);
        ziplist_repr(&zl);

        pop(&mut zl, ZIPLIST_TAIL);
        ziplist_repr(&zl);

        print!("[TEST]Get element at index 3: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(3);
            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                println!("ERROR: Could not access index 3");
                return;
            }
            println!("Get: {}, Expected: quux", entry);
            assert_eq!("quux", entry);
        }

        print!("[TEST]Get element at index 4: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(4);
            if pos == 0 {
                println!("No entry");
            } else {
                println!("ERROR: Out of range index should return NULL, returned offset: {pos}");
                return;
            }
        }
        print!("[TEST]Get element at index -1 (last element): ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(-1);
            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                println!("ERROR: Could not access index -1");
                return;
            }
            println!("Get: {}, Expected: quux", entry);
            assert_eq!("quux", entry);
        }

        print!("[TEST]Get element at index -4 (first element): ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(0);
            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                println!("ERROR: Could not access index -4");
                return;
            }
            println!("Get: {}, Expected: 1024", value);
            assert_eq!(1024, value);
        }

        print!("[TEST]Get element at index -5 (reverse out of range): ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(-5);
            if pos == 0 {
                println!("No entry");
            } else {
                println!("ERROR: Out of range index should return NULL, returned offset: {}", pos);
                return;
            }
        }

        print!("[TEST]Iterate list from 0 to end: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(0);
            while zl.get(pos, &mut entry, &mut elen, &mut value) {
                if !entry.is_empty() {
                    print!("{} ", entry);
                } else {
                    print!("{} ", value);
                }
                pos = zl.next_entry_position(pos);
            }
            print!("\n");
        }
        print!("[TEST]Iterate list from 1 to end: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(1);
            while zl.get(pos, &mut entry, &mut elen, &mut value) {
                if !entry.is_empty() {
                    if elen > 0 {
                        print!("{} ", entry);
                    }
                } else {
                    print!("{} ", value);
                }
                pos = zl.next_entry_position(pos);
            }
            print!("\n");
        }

        print!("[TEST]Iterate list from 2 to end: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(2);
            while zl.get(pos, &mut entry, &mut elen, &mut value) {
                if !entry.is_empty() {
                    if elen > 0 {
                        print!("{} ", entry);
                    }
                } else {
                    print!("{} ", value);
                }
                pos = zl.next_entry_position(pos);
            }
            print!("\n");
        }

        print!("[TEST]Iterate starting out of range: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(4);
            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                print!("No Entry");
            } else {
                print!("ERROR");
            }
            print!("\n");
        }

        print!("[TEST]Iterate from back to front: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(-1);
            while zl.get(pos, &mut entry, &mut elen, &mut value) {
                if !entry.is_empty() {
                    if elen > 0 {
                        print!("{} ", entry);
                    }
                } else {
                    print!("{} ", value);
                }
                pos = zl.prev_entry_position(pos);
            }
            print!("\n");
        }

        print!("[TEST]Iterate from back to front, deleting all items: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(-1);
            while zl.get(pos, &mut entry, &mut elen, &mut value) {
                if !entry.is_empty() {
                    if elen > 0 {
                        print!("{} ", entry);
                    }
                } else {
                    print!("{} ", value);
                }
                let _ = zl.delete(&mut pos);
                pos = zl.prev_entry_position(pos);
            }
            print!("\n");
        }

        print!("[TEST]Delete inclusive range (0, 0): ");
        {
            zl = create();
            let _ = zl.delete_range(0, 1);
            ziplist_repr(&zl);
        }

        print!("[TEST]Delete inclusive range (0, 1): ");
        {
            zl = create();
            let _ = zl.delete_range(0, 2);
            ziplist_repr(&zl);
        }

        print!("[TEST]Delete inclusive range (1, 2): ");
        {
            zl = create();
            let _ = zl.delete_range(1, 2);
            ziplist_repr(&zl);
        }

        print!("[TEST]Delete with start index out of range: ");
        {
            zl = create();
            let _ = zl.delete_range(5, 1);
            ziplist_repr(&zl);
        }

        print!("[TEST]Delete with num overflow: ");
        {
            zl = create();
            let _ = zl.delete_range(1, 5);
            ziplist_repr(&zl);
        }

        print!("[TEST]Delete foo while iterating: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;
            zl = create();
            pos = zl.zip_index(0);
            while zl.get(pos, &mut entry, &mut elen, &mut value) {
                if elen > 0 && entry == "foo" {
                    print!("[foo(delete)] ");
                    let _= zl.delete(&mut pos);
                } else {
                    if !entry.is_empty() {
                        print!("{} ", entry);
                    } else {
                        print!("{} ", value);
                    }
                    pos = zl.next_entry_position(pos);
                }
            }
            print!("\n");
            ziplist_repr(&zl);
        }

        print!("[TEST]Replace with same size: ");
        {
            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;

            zl = create();  /* "hello", "foo", "quux", "1024" */
            pos = zl.zip_index(0);
            let _ = zl.replace(pos, "zoink");
            pos = zl.zip_index(3);
            let _ = zl.replace(pos, "yy");
            pos = zl.zip_index(1);
            let _ = zl.replace(pos, "65536");
            pos = zl.zip_index(0);
            while zl.get(pos, &mut entry, &mut elen, &mut value) {
                if !entry.is_empty() {
                    if elen > 0 {
                        print!("{} ", entry);
                    }
                } else {
                    print!("{} ", value);
                }
                pos = zl.prev_entry_position(pos);
            }

            let expected: &[u8] = b"\x00\x05zoink\
                            \x07\xf0\x00\x00\x01\
                            \x05\x04quux\
                            \x06\x02yy\
                            \xff";
            //assert_eq!(&zl.data[pos..], expected);
            println!("SUCCESS");
        }
    }
}
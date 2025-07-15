#[cfg(test)]
mod test {
    use crate::data_structure::adlist::adlist::*;
    use crate::data_structure::ziplist::error::ZipListError;
    use crate::data_structure::ziplist::ziplist::{ZipList, ZlEntry};
    use crate::data_structure::ziplist::{
        ZIPLIST_HEAD, ZIPLIST_HEADER_SIZE, ZIPLIST_TAIL, ZIP_BIG_PREVLEN, ZIP_END,
    };

    use crate::data_structure::ziplist::lib::{
        store_entry_encoding, store_prev_entry_length, ziplist_merge, ziplist_repr,
        ziplist_valid_integerity,
    };
    use rand::Rng;
    use std::fmt::Write as _;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    fn create() -> ZipList {
        let mut zl = ZipList::new();
        let _ = zl.push("foo", false);
        let _ = zl.push("quux", false);
        let _ = zl.push("hello", true);
        let _ = zl.push("1024", false);
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

    fn stress(is_head: bool, num: usize, max_size: usize, dnum: usize) {
        for i in (0..max_size).step_by(dnum) {
            let mut zl = ZipList::new();

            for j in 0..i {
                let _ = zl.push("quux", is_head);
            }

            let start = Instant::now();
            for k in 0..num {
                let _ = zl.push("quux", is_head);
                let _ = zl.delete_range(0, 1);
            }
            let end = start.elapsed();
            println!(
                "List size: {i}, bytes: {}, push+pop: {}, time: {:?}",
                zl.ziplist_len(),
                num,
                end
            );
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
        let mut s = String::new();
        let (min_val, max_val) = match rand::rng().random::<u32>() % 3 {
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

    fn verify(zl: &mut ZipList, e: &mut [ZlEntry]) {
        let len = zl.entry_num() as i32;
        let entry = ZlEntry::default();

        for i in 0..len {
            e[i as usize] = zl.zip_entry(zl.zip_index(i));
            let _e = zl.zip_entry(zl.zip_index(i - len));
            assert_eq!(e[i as usize], _e);
        }
    }

    #[test]
    fn ziplist_test() {
        let mut pos = 0;
        let mut entry = String::default();
        let mut elen: u32 = 0;
        let mut value: i64 = 0;
        let iteration: i32 = 0;

        let mut zl = create_int_list();
        ziplist_repr(&mut zl);

        zl = create();
        ziplist_repr(&mut zl);

        pop(&mut zl, ZIPLIST_TAIL);
        ziplist_repr(&mut zl);

        pop(&mut zl, ZIPLIST_HEAD);
        ziplist_repr(&mut zl);

        pop(&mut zl, ZIPLIST_TAIL);
        ziplist_repr(&mut zl);

        pop(&mut zl, ZIPLIST_TAIL);
        ziplist_repr(&mut zl);

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
            println!("Get: {}, Expected: 1024", value);
            assert_eq!(1024, value);
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
            println!("Get: {}, Expected: 1024", value);
            assert_eq!(1024, value);
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
            println!("Get: {}, Expected: hello", entry);
            assert_eq!("hello", entry);
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
                println!(
                    "ERROR: Out of range index should return NULL, returned offset: {}",
                    pos
                );
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
            ziplist_repr(&mut zl);
        }

        print!("[TEST]Delete inclusive range (0, 1): ");
        {
            zl = create();
            let _ = zl.delete_range(0, 2);
            ziplist_repr(&mut zl);
        }

        print!("[TEST]Delete inclusive range (1, 2): ");
        {
            zl = create();
            let _ = zl.delete_range(1, 2);
            ziplist_repr(&mut zl);
        }

        print!("[TEST]Delete with start index out of range: ");
        {
            zl = create();
            let _ = zl.delete_range(5, 1);
            ziplist_repr(&mut zl);
        }

        print!("[TEST]Delete with num overflow: ");
        {
            zl = create();
            let _ = zl.delete_range(1, 5);
            ziplist_repr(&mut zl);
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
                    let _ = zl.delete(&mut pos);
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
            ziplist_repr(&mut zl);
        }

        print!("[TEST]Replace with same size: ");
        {
            let mut pos = 0;
            let entry = String::default();
            let elen: u32 = 0;
            let value: i64 = 0;

            zl = create(); /* "hello", "foo", "quux", "1024" */
            pos = zl.zip_index(0);
            let _ = zl.replace(pos, "zoink");
            pos = zl.zip_index(3);
            let _ = zl.replace(pos, "yy");
            pos = zl.zip_index(1);
            let _ = zl.replace(pos, "65536");
            pos = zl.zip_index(0);

            let expected: &[u8] = b"\x00\x05zoink\
                            \x07\xf0\x00\x00\x01\
                            \x05\x04quux\
                            \x06\x02yy\
                            \xff";
            assert_eq!(&zl.data[pos..], expected);
            println!("SUCCESS");
        }

        print!("[TEST]Replace with different size: ");
        {
            let mut pos = 0;
            zl = create();
            pos = zl.zip_index(1);
            let _ = zl.replace(pos, "squirrel");
            pos = zl.zip_index(0);
            let expected: &[u8] = b"\x00\x05hello\
                                    \x07\x08squirrel\
                                    \x0a\x04quux\
                                    \x06\xc0\x00\x04\
                                    \xff";
            assert_eq!(&zl.data[pos..], expected);
            println!("SUCCESS");
        }

        print!("[TEST]Regression test for >255 byte strings: ");
        {
            let mut v1 = [0; 257];
            let mut v2 = [0; 257];
            v1[..256].fill(b'x');
            v2[..256].fill(b'y');

            let mut pos = 0;
            let mut entry = String::default();
            let mut elen: u32 = 0;
            let mut value: i64 = 0;

            let mut zl = ZipList::new();
            let _ = zl.push(std::str::from_utf8(&v1).unwrap(), false);
            let _ = zl.push(std::str::from_utf8(&v2).unwrap(), false);

            pos = zl.zip_index(0);
            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                panic!("FAIL");
            }
            assert_eq!(&v1, entry.as_bytes());
            pos = zl.zip_index(1);

            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                panic!("FAIL");
            }
            assert_eq!(&v2, entry.as_bytes());
            println!("SUCCESS");
        }

        print!("[TEST]Regression test deleting next to last entries: ");
        {
            let mut v = [[0; 257]; 3];
            let mut e = [ZlEntry::default(); 3];

            for (i, row) in v.iter_mut().enumerate() {
                row.fill(b'a' + i as u8);
            }
            v[0][256] = 0;
            v[1][1] = 0;
            v[2][256] = 0;

            let mut zl = ZipList::new();
            for (i, row) in v.iter().enumerate() {
                let _ = zl.push(std::str::from_utf8(&v[i]).unwrap(), false);
            }

            verify(&mut zl, &mut e);
            assert_eq!(e[0].prev_raw_len_size, 1);
            assert_eq!(e[1].prev_raw_len_size, 5);
            assert_eq!(e[2].prev_raw_len_size, 5);

            let mut pos = e[1].pos;
            let _ = zl.delete(&mut pos);

            verify(&mut zl, &mut e);
            assert_eq!(e[0].prev_raw_len_size, 1);
            assert_eq!(e[1].prev_raw_len_size, 5);
            println!("SUCCESS");
        }

        print!("[TEST]Create long list and check indices: ");
        {
            let start = Instant::now();
            let mut zl = ZipList::new();

            for i in 0..1000 {
                let _ = zl.push(&i.to_string(), false);
            }

            for i in 0..1000 {
                let mut pos = zl.zip_index(i);
                assert_eq!(
                    true,
                    zl.get(pos, &mut String::default(), &mut 0, &mut value)
                );
                assert_eq!(i as i64, value);

                pos = zl.zip_index(-i - 1);
                assert_eq!(
                    true,
                    zl.get(pos, &mut String::default(), &mut 0, &mut value)
                );
                assert_eq!(999 - i as i64, value);
            }

            let end = start.elapsed();
            println!("SUCCESS, time = {:?}", end);
        }

        print!("[TEST]Compare strings with ziplist entries: ");
        {
            let zl = create();
            let mut pos = zl.zip_index(0);
            if !zl.compare(pos, &"hello") {
                panic!("ERROR: NOT hello");
            }
            if zl.compare(pos, &"hella") {
                panic!("ERROR: hella");
            }

            pos = zl.zip_index(3);
            if !zl.compare(pos, &"1024") {
                panic!("ERROR: NOT 1024");
            }
            if zl.compare(pos, &"1025") {
                panic!("ERROR: 1025");
            }
            println!("SUCCESS");
        }

        print!("[TEST]Merge test: ");
        {
            /* create list gives us: [hello, foo, quux, 1024] */
            let zl = create();
            let zl2 = create();
            let zl3 = ZipList::new();
            let zl4 = ZipList::new();

            if let Some(mut zl4) = ziplist_merge(&mut Some(zl3), &mut Some(zl4)) {
                ziplist_repr(&mut zl4);
            }

            /* merge gives us: [hello, foo, quux, 1024, hello, foo, quux, 1024] */
            let mut zl2 = ziplist_merge(&mut Some(zl), &mut Some(zl2)).unwrap();
            if zl2.entry_num() != 8 {
                panic!("ERROR: Merged length not 8, but: {}", zl2.entry_num());
            }
            let pos = zl2.zip_index(0);
            if !zl2.compare(pos, "hello") {
                panic!("ERROR: not hello");
            }
            if zl2.compare(pos, "hella") {
                panic!("ERROR: hella");
            }

            let pos = zl2.zip_index(3);
            if !zl2.compare(pos, "1024") {
                panic!("ERROR: not 1024");
            }
            if zl2.compare(pos, "1025") {
                panic!("ERROR: 1025");
            }

            let pos = zl2.zip_index(4);
            if !zl2.compare(pos, "hello") {
                panic!("ERROR: not hello");
            }
            if zl2.compare(pos, "hella") {
                panic!("ERROR: hella");
            }

            let pos = zl2.zip_index(7);
            if !zl2.compare(pos, "1024") {
                panic!("ERROR: not 1024");
            }
            if zl2.compare(pos, "1025") {
                panic!("ERROR: 1025");
            }
            print!("SUCCESS");
        }
    }
    #[test]
    fn stress_test() -> Result<(), ZipListError> {
        print!("[TEST]Stress with random payloads of different encoding: ");
        {
            let start = Instant::now();
            let mut buf_len = 1024;
            let mut buf = String::with_capacity(1024);
            let mut list_value = 0;
            let iteration = 200;

            for i in 0..iteration {
                let mut zl = ZipList::new();
                let mut list = LinkList::create();
                let len = rand::rng().random::<u32>() % 256;

                for j in 0..len {
                    let is_head = if rand::rng().random::<u32>() & 1 == 1 {
                        true
                    } else {
                        false
                    };
                    if rand::rng().random::<u32>() % 2 == 1 {
                        buf = rand_string(1, buf_len - 1);
                    } else {
                        match rand::rng().random::<u32>() % 3 {
                            0 => {
                                list_value = (rand::rng().random::<u32>() as i64) >> 20;
                                let _ = write!(&mut buf, "{}", list_value);
                            }
                            1 => {
                                list_value = rand::rng().random::<u32>() as i64;
                                let _ = write!(&mut buf, "{}", list_value);
                            }
                            2 => {
                                list_value = (rand::rng().random::<u32>() as i64) << 20;
                                let _ = write!(&mut buf, "{}", list_value);
                            }
                            _ => {}
                        }
                    }
                    /* Add to ziplist */
                    zl.push(&buf, is_head)?;
                    if is_head {
                        list.add_node_head(buf.clone());
                    } else {
                        list.add_node_tail(buf.clone());
                    }
                }
                assert_eq!(zl.entry_num(), list.length() as u32);
                for j in 0..len {
                    let mut entry = String::default();
                    let mut elen: u32 = 0;
                    let mut value: i64 = 0;
                    let pos = zl.zip_index(j as i32);
                    let list_node = list.index(j as i64);

                    assert_eq!(true, zl.get(pos, &mut entry, &mut elen, &mut value));
                    if entry.is_empty() {
                        unsafe {
                            let v = (*list_node.unwrap().as_ptr()).value();
                            assert_eq!(value.to_string(), *v);
                        }
                    } else {
                        unsafe {
                            let v = (*list_node.unwrap().as_ptr()).value();
                            assert_eq!(entry, *v);
                        }
                    }
                }
            }
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        print!("Stress with variable ziplist size: ");
        {
            let start = Instant::now();
            let maxsize = 16384;
            stress(true, 100000, maxsize, 256);
            stress(false, 100000, maxsize, 256);
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        Ok(())
    }

    #[test]
    fn benchmark() -> Result<(), ZipListError> {
        let mut zl = ZipList::new();
        let iteration = 1000;
        for i in 0..iteration {
            let mut buf = vec![0u8; 4096];
            buf[..4].copy_from_slice(b"asdf");
            let s = String::from_utf8(buf).unwrap();
            zl.push(&s[0..4], false)?;
            zl.push(&s[0..40], false)?;
            zl.push(&s[0..400], false)?;
            zl.push(&s[0..4000], false)?;
            zl.push("1", false)?;
            zl.push("10", false)?;
            zl.push("100", false)?;
            zl.push("1000", false)?;
            zl.push("10000", false)?;
            zl.push("100000", false)?;
        }

        print!("benchmark ziplist Find: ");
        {
            let start = Instant::now();
            for i in 0..2000 {
                let fptr = zl.zip_index(0);
            }
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        print!("Benchmark ziplist Index: ");
        {
            let start = Instant::now();
            for i in 0..2000 {
                zl.zip_index(999);
            }
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        print!("Benchmark ziplist Validate Integrity: ");
        {
            let start = Instant::now();
            for i in 0..2000 {
                let len = zl.ziplist_len();
                ziplist_valid_integerity(&mut zl, len, 1, None, None);
            }
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        print!("Benchmark ziplist Compare with string: ");
        {
            let start = Instant::now();
            for i in 0..2000 {
                let mut pos = zl.zip_index(0);
                while zl.data[pos] != ZIP_END {
                    zl.compare(pos, "nothing");
                    pos = zl.next_entry_position(pos);
                }
            }
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        print!("Benchmark ziplist Compare with number: ");
        {
            let start = Instant::now();
            for i in 0..2000 {
                let mut pos = zl.zip_index(0);
                while zl.data[pos] != ZIP_END {
                    zl.compare(pos, "99999");
                    pos = zl.next_entry_position(pos);
                }
            }
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        Ok(())
    }

    #[test]
    fn ziplist_cascade_update() -> Result<(), ZipListError> {
        print!("Stress ziplist CascadeUpdate: ");
        {
            let buf = vec![b'0'; ZIP_BIG_PREVLEN as usize];
            let s = String::from_utf8(buf).unwrap();
            let mut zl = ZipList::new();
            let iteration = 1000;
            for i in 0..iteration {
                zl.push(&s[..ZIP_BIG_PREVLEN as usize - 4], false)?;
            }
            let start = Instant::now();
            zl.push(&s[..ZIP_BIG_PREVLEN as usize - 3], true)?;
            let end = start.elapsed();
            println!("Done, time = {:?}", end);
        }

        fn insert_helper(
            zl: &mut ZipList,
            ch: char,
            len: usize,
            pos: usize,
        ) -> Result<(), ZipListError> {
            assert!(len <= ZIP_BIG_PREVLEN as usize);
            let mut data = vec![b'0'; ZIP_BIG_PREVLEN as usize];
            data[..len].fill(u8::try_from(ch).unwrap());
            let s = String::from_utf8(data).unwrap();
            zl.insert(pos, &s[..len])
        }

        fn compare_help(zl: &ZipList, ch: char, len: usize, index: i32) -> bool {
            assert!(len <= ZIP_BIG_PREVLEN as usize);
            let mut data = vec![b'0'; ZIP_BIG_PREVLEN as usize];
            data[..len].fill(u8::try_from(ch).unwrap());
            let s = String::from_utf8(data).unwrap();
            let pos = zl.zip_index(index);
            assert_ne!(zl.data[pos], ZIP_END);
            zl.compare(pos, &s[..len])
        }

        fn str_entry_bytes_small(len: usize) -> usize {
            len + store_prev_entry_length(None, 0) as usize
                + store_entry_encoding(None, 0, len as u32) as usize
        }

        fn str_entry_bytes_big(len: usize) -> usize {
            len + store_prev_entry_length(None, ZIP_BIG_PREVLEN as u32) as usize
                + store_entry_encoding(None, 0, len as u32) as usize
        }

        println!("Edge cases of ziplist CascadeUpdate: ");
        {
            let mut zl = ZipList::new();
            let s1 = ZIP_BIG_PREVLEN as usize - 4;
            let s2 = ZIP_BIG_PREVLEN as usize - 3;
            let mut e = [ZlEntry::default(); 4];

            insert_helper(&mut zl, 'a', s1, ZIPLIST_HEADER_SIZE as usize)?;
            verify(&mut zl, &mut e);

            assert!(e[0].prev_raw_len_size == 1 && e[0].prev_raw_len == 0);
            assert!(compare_help(&zl, 'a', s1, 0));

            ziplist_repr(&mut zl);

            /* No expand. */
            insert_helper(&mut zl, 'b', s1, ZIPLIST_HEADER_SIZE as usize)?;
            verify(&mut zl, &mut e);

            assert!(e[0].prev_raw_len_size == 1 && e[0].prev_raw_len == 0);
            assert!(compare_help(&zl, 'b', s1, 0));

            assert!(
                e[1].prev_raw_len_size == 1
                    && e[1].prev_raw_len == str_entry_bytes_small(s1) as u32
            );
            assert!(compare_help(&zl, 'a', s1, 1));

            ziplist_repr(&mut zl);

            // Expand(tail included).
            insert_helper(&mut zl, 'c', s2, ZIPLIST_HEADER_SIZE as usize)?;
            verify(&mut zl, &mut e);

            assert!(e[0].prev_raw_len_size == 1 && e[0].prev_raw_len == 0);
            assert!(compare_help(&zl, 'c', s2, 0));

            assert!(
                e[1].prev_raw_len_size == 5
                    && e[1].prev_raw_len == str_entry_bytes_small(s2) as u32
            );
            assert!(compare_help(&zl, 'b', s1, 1));

            assert!(
                e[2].prev_raw_len_size == 5 && e[2].prev_raw_len == str_entry_bytes_big(s1) as u32
            );
            assert!(compare_help(&zl, 'a', s1, 2));

            ziplist_repr(&mut zl);

            // Expand(only previous head entry)
            insert_helper(&mut zl, 'd', s2, ZIPLIST_HEADER_SIZE as usize)?;
            verify(&mut zl, &mut e);

            assert!(e[0].prev_raw_len_size == 1 && e[0].prev_raw_len == 0);
            assert!(compare_help(&zl, 'd', s2, 0));

            assert!(
                e[1].prev_raw_len_size == 5
                    && e[1].prev_raw_len == str_entry_bytes_small(s2) as u32
            );
            assert!(compare_help(&zl, 'c', s2, 1));

            assert!(
                e[2].prev_raw_len_size == 5 && e[2].prev_raw_len == str_entry_bytes_big(s2) as u32
            );
            assert!(compare_help(&zl, 'b', s1, 2));

            assert!(
                e[3].prev_raw_len_size == 5 && e[3].prev_raw_len == str_entry_bytes_big(s1) as u32
            );
            assert!(compare_help(&zl, 'a', s1, 3));

            ziplist_repr(&mut zl);

            // Delete from mid
            let mut pos = zl.zip_index(2);
            zl.delete(&mut pos)?;
            verify(&mut zl, &mut e);

            assert!(e[0].prev_raw_len_size == 1 && e[0].prev_raw_len == 0);
            assert!(compare_help(&zl, 'd', s2, 0));

            assert!(
                e[1].prev_raw_len_size == 5
                    && e[1].prev_raw_len == str_entry_bytes_small(s2) as u32
            );
            assert!(compare_help(&zl, 'c', s2, 1));

            assert!(
                e[2].prev_raw_len_size == 5 && e[2].prev_raw_len == str_entry_bytes_big(s2) as u32
            );
            assert!(compare_help(&zl, 'a', s1, 2));

            ziplist_repr(&mut zl);
        }

        Ok(())
    }

    #[test]
    fn ziplist_insert_nextdiff() -> Result<(), ZipListError> {
        print!("ziplist Insert nextdiff == -4 && reqlen < 4 (issue #7170):");
        {
            let mut zl = ZipList::new();

            // We set some values to almost reach the critical point - 254
            let mut buf_252 = vec![b'0'; 252];
            let mut buf_250 = vec![b'0'; 250];
            buf_252.fill(u8::try_from('A').unwrap());
            buf_250.fill(u8::try_from('A').unwrap());
            let s_252 = String::from_utf8(buf_252).unwrap();
            let s_250 = String::from_utf8(buf_250).unwrap();

            // After the rpush, the list look like: [one two A_252 A_250 three 10]
            zl.push("one", false)?;
            zl.push("two", false)?;
            zl.push(&s_252, false)?;
            zl.push(&s_250, false)?;
            zl.push("three", false)?;
            zl.push("10", false)?;
            ziplist_repr(&mut zl);

            let mut pos = zl.zip_index(2);
            if !zl.compare(pos, &s_252) {
                panic!("ERROR: not \"A_252\"");
            }

            zl.delete(&mut pos)?;
            ziplist_repr(&mut zl);

            pos = zl.zip_index(3);
            if !zl.compare(pos, "three") {
                panic!("ERROR: not \"three");
            }

            zl.insert(pos, "10")?;
            ziplist_repr(&mut zl);

            pos = zl.zip_index(-1);
            if !zl.compare(pos, "10") {
                panic!("ERROR: not \"10");
            }
        }
        print!("ALL TESTS PASSED!");
        Ok(())
    }
}

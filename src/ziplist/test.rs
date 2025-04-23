
#[cfg(test)]
mod test {
    use crate::ziplist::error::ZipListError;
    use crate::ziplist::ziplist::ZipList;
    use crate::ziplist::{ZIPLIST_HEAD, ZIPLIST_HEADER_SIZE};

    use std::io::{self, Write};
    use std::time::{SystemTime, UNIX_EPOCH, Instant};

    fn create() -> ZipList {
        let mut zl = ZipList::new();
        zl.push("foo", false);
        zl.push("quux", false);
        zl.push("hello", true);
        zl.push("1024", true);
        zl
    }

    fn create_int_list() -> ZipList {
        let mut zl = ZipList::new();
        zl.push("100", false);
        zl.push("12800", false);
        zl.push("-100", true);
        zl.push("4294967296", true);
        zl.push("non integer", false);
        zl.push("much much longer non integer", false);
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
                zl.push("quux", false);
            }

            let start = Instant::now();
            for k in 0..num {
                zl.push("quux", false);
                zl.delete_range(0, 1);
            }
            let end = start.elapsed();
            println!("List size: {i}, bytes: {}, push+pop: {}, time: {:?}", zl.ziplist_len(), num, end);
        }
    }

    fn pop(zl: ZipList, where_: i32) {
        let p = if where_ == ZIPLIST_HEAD {
            zl.zip_index(0)
        } else {
            zl.zip_index(-1)
        };
        let mut vlen: u32 = 0;
        let mut vstr = String::default();
        let mut vlong: i64 = 0;
        if zl.get(p, &mut vstr, &mut vlen, &mut vlong) {
            if where_ == ZIPLIST_HEAD {
                println!("Pop head: ");
            } else {
                println!("Pop tail ");
            }
        }
    }

    #[test]
    fn ziplist_test() {
        let mut zl = ZipList::new();
        let mut pos = 0;
        let mut entry = String::default();
        let mut elen: u32 = 0;
        let mut value: i64 = 0;
        let mut iteration: i32 = 0;
        println!("Get element at index 3: ");
        {
            zl = ZipList::new();
            pos = zl.zip_index(3);
            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                println!("ERROR: Could not access index 3");
                return;
            }
            if entry != String::default() {

            } else {
                println!("value: {value}");
            }
        }
        println!("Get element at index 4");
        {
            zl = ZipList::new();
            pos = zl.zip_index(4);
            if pos == 0 {
                println!("No entry");
            } else {
                println!("ERROR: Out of range index should return NULL, returned offset: {pos}");
                return;
            }
        }
        println!("Get element at index -1 (last element)");
        {
            zl = ZipList::new();
            pos = zl.zip_index(-1);
            if !zl.get(pos, &mut entry, &mut elen, &mut value) {
                println!("ERROR: Could not access index -1");
                return;
            }
            if !entry.is_empty() {
                if elen != 0 {
                    println!(" entry: {entry}");
                }
            } else {
                println!("value: {value}");
            }
        }
    }

    #[test]
    fn ziplist_insert() -> Result<(), ZipListError> {
        let mut zl = ZipList::new();
        let num = 5;
        for i in 0..num {
            let s = "1000";
            zl.push(s, false)?;
        }
        let entry_num = zl.entry_num();
        println!("entry num: {entry_num}");
        println!("tail offset: {}", zl.tail_offset());
        println!("ziplist len: {}", zl.ziplist_len());

        let mut p = ZIPLIST_HEADER_SIZE as usize;
        while let Some(next) = zl.next_entry_position(p) {
            //println!("next pos: {}", next);
            p = next;
            if let Some(entry) = zl.zip_get_entry(next) {
                println!("next entry: {:?}", entry);
            }
        }
        let mut p = zl.ziplist_len() - 1;
        while let Some(prev) = zl.prev_entry_position(p) {
            //println!("prev pos: {}", next);
            p = prev;
            if let Some(entry) = zl.zip_get_entry(prev) {
                println!("prev entry: {:?}", entry);
            }
        }
        Ok(())
    }

    #[test]
    fn ziplist_delete() -> Result<(), ZipListError> {
        let mut zl = ZipList::new();
        let num = 5;
        for i in 0..num {
            let s = "1000";
            zl.push(s, false)?;
        }
        let entry_num = zl.entry_num();
        println!("entry num: {entry_num}");
        println!("tail offset: {}", zl.tail_offset());
        println!("ziplist len: {}", zl.ziplist_len());

        let mut p = ZIPLIST_HEADER_SIZE as usize;
        let res = zl._delete(p, 3);

        while let Some(next) = zl.next_entry_position(p) {
            //println!("next pos: {}", next);
            p = next;
            if let Some(entry) = zl.zip_get_entry(next) {
                println!("next entry: {:?}", entry);
            }
        }
        let entry_num = zl.entry_num();
        println!("entry num: {entry_num}");
        println!("tail offset: {}", zl.tail_offset());
        println!("ziplist len: {}", zl.ziplist_len());

        Ok(())
    }
}
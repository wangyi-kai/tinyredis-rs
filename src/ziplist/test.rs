
#[cfg(test)]
mod test {
    use crate::ziplist::error::ZipListError;
    use crate::ziplist::ziplist::ZipList;
    use crate::ziplist::ZIPLIST_HEADER_SIZE;

    use rand::distr::Alphanumeric;
    use rand::{rng, Rng};

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
        let entry = zl.zip_get_entry(10).unwrap();

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
}
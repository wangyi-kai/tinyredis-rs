
#[cfg(test)]
mod test {
    use crate::ziplist::ziplist::ZipList;

    #[test]
    fn ziplist_insert() {
        let mut zl = ZipList::new();
        let num = 2000;
        for i in 0..num {
            let entry = "1000";
            let res = zl.push(&entry, 0);
            match res {
                Ok(_) => {
                    println!("Insert success");
                }
                Err(e) => {
                    println!("Insert Error: {:?}", e);
                }
            }
        }
    }
}
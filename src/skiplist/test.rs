
#[cfg(test)]
mod skiplist_test {
    use crate::skiplist::skiplist::SkipList;

    #[test]
    fn skiplist_insert() {
        let mut skip_list = unsafe { SkipList::new() };
        let num = 10;
        for i in 0..num {
            let elem = "i".to_string();
            unsafe { skip_list.insert(i as f64, elem); }
        }
    }

    #[test]
    fn skiplist_find() {
        let mut skip_list = unsafe { SkipList::new() };
        let num = 100i64;
        for i in 0..num {
            let elem = i.to_string();
            skip_list.insert(i as f64, elem);
        }
        unsafe {
            for i in 0..num {
                let x = skip_list.get_elem_by_rank(i + 1);
                if let Some(x) = x {
                    let elem = (*x.as_ptr()).get_elem();
                    println!("elem: {}", elem);
                }
            }
        }
    }

    #[test]
    fn skiplist_delete() {
        let mut skip_list = unsafe { SkipList::new() };
        let num = 10000;
        for i in 0..num {
            let elem = i.to_string();
            unsafe { skip_list.insert(i as f64, elem); }
        }

        unsafe {
            for i in 0..num {
                let elem = i.to_string();
                skip_list.delete(i as f64, &elem);
            }
            for i in 0..num {
                let x = skip_list.get_elem_by_rank(i + 1);
                if let Some(x) = x {
                    let elem = (*x.as_ptr()).get_elem();
                    println!("elem: {}", elem);
                }
            }
        }
    }

    #[test]
    fn skiplist_update() {
        let mut skip_list = unsafe { SkipList::new() };
        let num = 100;
        for i in 0..num {
            let elem = i.to_string();
            unsafe { skip_list.insert(i as f64, elem); }
        }
        for i in 0..num {
            let elem = i.to_string();
            unsafe {
                skip_list.update_score(i as f64, &elem, 100f64);
                let x = skip_list.get_elem_by_rank(i + 1);
                if let Some(x) = x {
                    let score = (*x.as_ptr()).get_score();
                    println!("score: {}", score);
                    //assert_eq!(score, 100);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::adlist::adlist::List;

    #[test]
    fn list_insert() {
        let mut list = List::create();
        let num = 10;

        for i in 0..num {
            let s = i.to_string();
            list.add_node_head(s);
        }
        for i in 0..num {
            let s = i.to_string();
            list.add_node_tail(s);
        }

        let iter = list.iter();
        for v in iter {
            print!("{} ", v);
        }
    }

    #[test]
    fn list_delete() {
        let mut list = List::create();
        let num = 10;

        for i in 0..num {
            let s = i.to_string();
            list.add_node_tail(s);
        }

        for i in 0..num {
            if i % 2 == 1 {
                let s = i.to_string();
                list.delete(s);
            }
        }

        let iter = list.iter();
        for v in iter {
            print!("{} ", v);
        }
    }
}
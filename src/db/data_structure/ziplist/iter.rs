use crate::db::data_structure::ziplist::ziplist::ZipList;

pub struct ZipListIter<'a> {
    cur: usize,
    ziplist: &'a ZipList,
}

impl ZipList {
    pub fn iter(&self) -> ZipListIter {
        ZipListIter {
            cur: 0,
            ziplist: self,
        }
    }
}

impl Iterator for ZipListIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur == 0 {
            self.cur = self.ziplist.zip_index(0);
            return Some(self.cur);
        }
        let next = self.ziplist.next_entry_position(self.cur);
        if next == 0 {
            return None;
        }
        self.cur = next;
        return Some(next);
    }
}

pub struct HashTypeIter<'a> {
    pub field_pos: usize,
    pub value_pos: usize,
    subject: &'a ZipList,
}

impl ZipList {
    pub fn hash_iter(&self) -> HashTypeIter {
        HashTypeIter {
            field_pos: 0,
            value_pos: 0,
            subject: self,
        }
    }
}

impl Iterator for HashTypeIter<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.field_pos == 0 {
            self.field_pos = self.subject.zip_index(0);
        } else {
            self.field_pos = self.subject.next_entry_position(self.value_pos);
        }
        if self.field_pos == 0 {
            return None;
        }
        self.value_pos = self.subject.next_entry_position(self.field_pos);
        Some(true)
    }
}
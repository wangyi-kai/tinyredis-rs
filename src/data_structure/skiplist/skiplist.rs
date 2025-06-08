use std::ptr::NonNull;
use super::{SKIP_LIST_MAX_LEVEL};
use super::lib::{random_level, sds_cmp};

#[derive(Clone, Debug, Default)]
pub struct Node {
    elem: String,
    score: f64,
    backward: Option<NonNull<Node>>,
    pub level: Vec<Level>,
}

impl Node {
    pub fn get_elem(&self) -> String {
        self.elem.clone()
    }

    pub fn get_score(&self) -> f64 {
        self.score
    }
}

impl Node {
    pub fn new(elem: String, score: f64, level: usize) -> Self {
        Self {
            elem,
            score,
            backward: None,
            level: vec![Level::new(0); level],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Level {
    pub forward: Option<NonNull<Node>>,
    span: u64,
}

impl Level {
    pub fn new_with_node(span: u64, forward: Node) -> Self {
        unsafe {
            Self {
                forward: Some(NonNull::new_unchecked(Box::into_raw(Box::new(forward)))),
                span,
            }
        }
    }

    pub fn new(span: u64) -> Self {
        Self {
            forward: None,
            span,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkipList {
    /// head node
    pub head: Option<NonNull<Node>>,
    /// tail node
    pub tail: Option<NonNull<Node>>,
    /// number of nodes in skip_list
    length: u64,
    /// level of node with max level
    level: usize,
}

impl SkipList {
    pub fn new() -> Self {
        unsafe {
            let head = Node::new(String::default(), 0f64, SKIP_LIST_MAX_LEVEL);

            Self {
                head: Some(NonNull::new_unchecked(Box::into_raw(Box::new(head)))),
                tail: None,
                length: 0,
                level: 1,
            }
        }
    }

    #[inline(always)]
    pub fn insert(&mut self, score: f64, elem: String) -> NonNull<Node> {
        unsafe {
            let mut update = vec![NonNull::new_unchecked(Box::into_raw(Box::new(Node::default()))); SKIP_LIST_MAX_LEVEL];
            let mut rank = vec![0u64; SKIP_LIST_MAX_LEVEL];
            let mut x = self.head.unwrap();
            assert!(!score.is_nan());

            for i in (0..self.level).rev() {
                // store rank that is crossed to reach the insert position
                rank[i] = if i == (self.level - 1) { 0 } else { rank[i + 1] };
                while let Some(level) = (*x.as_ptr()).level.get(i) {
                    if let Some(forward) = level.forward {
                        let l_score = (*forward.as_ptr()).score;
                        let l_elem = &(*forward.as_ptr()).elem;
                        if l_score < score || (l_score == score && sds_cmp(l_elem, &elem) < 0) {
                            rank[i] += level.span;
                            x = forward;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                update[i] = x;
            }

            let level = random_level();
            if level > self.level {
                for i in self.level..level {
                    rank[i] = 0;
                    update[i] = self.head.unwrap();
                    (*update[i].as_ptr()).level[i].span = self.length;
                }
                self.level = level;
            }
            let x = Box::into_raw(Box::new(Node::new(elem, score, level)));
            for i in 0..level {
                (*x).level[i].forward = (*update[i].as_ptr()).level[i].forward;
                (*update[i].as_ptr()).level[i].forward = Some(NonNull::new_unchecked(x));
                (*x).level[i].span = (*update[i].as_ptr()).level[i].span - (rank[0] - rank[i]);
                (*update[i].as_ptr()).level[i].span = (rank[0] - rank[i]) + 1;
            }

            for i in level..self.level {
                (*update[i].as_ptr()).level[i].span += 1;
            }
            if update[0] == self.head.unwrap() {
                (*x).backward = None;
            } else {
                Some(update[0]);
            }
            if (*x).level[0].forward.is_some() {
                let forward = (*x).level[0].forward.unwrap();
                (*forward.as_ptr()).backward = Some(NonNull::new_unchecked(x));
            } else {
                self.tail = Some(NonNull::new_unchecked(x));
            }
            self.length += 1;
            NonNull::new_unchecked(x)
        }
    }

    #[inline(always)]
    fn delete_node(&mut self, x: NonNull<Node>, update: &Vec<NonNull<Node>>) {
        unsafe {
            for i in 0..self.level {
                if let Some(forward) = (*update[i].as_ptr()).level[i].forward {
                    if forward == x {
                        (*update[i].as_ptr()).level[i].span += (*x.as_ptr()).level[i].span - 1;
                        (*update[i].as_ptr()).level[i].forward = (*x.as_ptr()).level[i].forward;
                    } else {
                        (*update[i].as_ptr()).level[i].span -= 1;
                    }
                } else {
                    continue;
                }
            }

            if let Some(forward) = (*x.as_ptr()).level[0].forward {
                (*forward.as_ptr()).backward = (*x.as_ptr()).backward;
            } else {
                self.tail = (*x.as_ptr()).backward;
            }
            let head = self.head.unwrap();
            while self.level > 1 && (*head.as_ptr()).level[self.level - 1].forward.is_none() {
                self.level -= 1;
            }
            self.length -= 1;
        }
    }

    #[inline(always)]
    pub fn delete(&mut self, score: f64, elem: &String) -> bool {
        unsafe {
            let mut update = vec![NonNull::new_unchecked(Box::into_raw(Box::new(Node::default()))); SKIP_LIST_MAX_LEVEL];
            let mut x = self.head.unwrap();

            for i in (0..self.level).rev() {
                while let Some(forward) = (*x.as_ptr()).level[i].forward {
                    let l_score = (*forward.as_ptr()).score;
                    let l_elem = &(*forward.as_ptr()).elem;
                    if l_score < score || (l_score == score && sds_cmp(l_elem, &elem) < 0) {
                        x = forward;
                    } else {
                        break;
                    }
                }
                update[i] = x;
            }

            let forward = (*x.as_ptr()).level[0].forward;
            if let Some(forward) = forward {
                x = forward;
                return if score == (*x.as_ptr()).score && sds_cmp(&(*x.as_ptr()).elem, elem) == 0 {
                    self.delete_node(x, &update);
                    true
                } else {
                    false
                }
            }
            false
        }
    }

    #[inline(always)]
    pub fn update_score(&mut self, cur_score: f64, elem: &str, new_score: f64) -> NonNull<Node> {
        unsafe {
            let mut update = vec![NonNull::new_unchecked(Box::into_raw(Box::new(Node::default()))); SKIP_LIST_MAX_LEVEL];
            let mut x = self.head.unwrap();

            for i in (0..self.level).rev() {
                while let Some(forward) = (*x.as_ptr()).level[i].forward {
                    let l_score = (*forward.as_ptr()).score;
                    let l_elem = &(*forward.as_ptr()).elem;
                    if l_score < cur_score || (l_score == cur_score && sds_cmp(l_elem, elem) < 0) {
                        x = forward;
                    } else {
                        break;
                    }
                }
                update[i] = x;
            }

            x = (*x.as_ptr()).level[0].forward.unwrap();
            if ((*x.as_ptr()).backward.is_none() || (*(*x.as_ptr()).backward.unwrap().as_ptr()).score < new_score) && ((*x.as_ptr()).level[0].forward.is_none() || (*(*x.as_ptr()).level[0].forward.unwrap().as_ptr()).score > new_score) {
                (*x.as_ptr()).score = new_score;
                return x;
            }
            self.delete_node(x, &update);
            let new_node = self.insert(new_score, (*x.as_ptr()).elem.clone());
            (*x.as_ptr()).elem = String::default();
            new_node
        }
    }

    #[inline(always)]
    pub fn get_rank(&self, score: f64, elem: &String) -> i64 {
        unsafe {
            let mut rank = 0;
            let mut x = self.head.unwrap();

            for i in (0..self.level - 1).rev() {
                while let Some(level) = (*x.as_ptr()).level.get(i) {
                    if let Some(forward) = level.forward {
                        let l_score = (*forward.as_ptr()).score;
                        let l_elem = &(*forward.as_ptr()).elem;
                        if l_score < score || (l_score == score && sds_cmp(l_elem, &elem) < 0) {
                            rank += level.span;
                            x = forward;
                        }
                    }
                }

                if (*x.as_ptr()).elem != String::default() && (*x.as_ptr()).score == score && sds_cmp(&(*x.as_ptr()).elem, elem) == 0 {
                    return rank as i64;
                }
            }
        }
        0
    }

    #[inline(always)]
    pub fn get_elem_by_rank(&self, rank: i64) -> Option<NonNull<Node>> {
        unsafe {
            let mut x = self.head.unwrap();
            let mut traversed = 0;
            for i in (0..self.level).rev() {
                while (*x.as_ptr()).level[i].forward.is_some() && ((*x.as_ptr()).level[i].span + traversed) <= rank as u64 {
                    traversed += (*x.as_ptr()).level[i].span;
                    x = (*x.as_ptr()).level[i].forward.unwrap();
                }
                if traversed == rank as u64 {
                    return Some(x);
                }
            }
            None
        }
    }
}

impl Drop for SkipList {
    fn drop(&mut self) {
        unsafe {
            let mut node = (*self.head.unwrap().as_ptr()).level[0].forward;
            while node.is_some() {
                let box_node = Box::from_raw(node.unwrap().as_ptr());
                let next = (*node.unwrap().as_ptr()).level[0].forward;
                node = next;
            }
        }
    }
}
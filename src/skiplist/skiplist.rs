use std::cmp::Ordering;
use std::ptr::NonNull;
use crate::skiplist::{RAND_MAX, SKIP_LIST_MAX_LEVEL, SKIP_LIST_P, SKIP_MAX_SEARCH};

use rand::Rng;

pub fn gen_random() -> u32 {
    let mut rng = rand::rng();
    rng.random::<u32>()
}

pub fn random_level() -> usize {
    let threshold = SKIP_LIST_P * RAND_MAX as f32;
    let mut level = 1;
    while (gen_random() as f32) < threshold {
        level += 1;
    }
    level.min(SKIP_LIST_MAX_LEVEL as usize)
}

fn sds_cmp(s1: &str, s2: &str) -> i32 {
    let l1 = s1.len();
    let l2 = s2.len();
    let min_len = if l1 < l2 { l1 } else { l2 };
    let order = s1[..min_len].cmp(&s2[..min_len]);
    match order {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    elem: String,
    score: i64,
    backward: Option<NonNull<Node>>,
    pub level: Vec<Level>,
}

impl Node {
    pub fn get_elem(&self) -> String {
        self.elem.clone()
    }

    pub fn get_score(&self) -> i64 {
        self.score
    }
}

impl Node {
    pub fn new(elem: String, score: i64, level: usize) -> Self {
        Self {
            elem,
            score,
            backward: None,
            level: vec![Level::new(0); level],
        }
    }
}

#[derive(Clone, Debug)]
struct Level {
    pub forward: Option<NonNull<Node>>,
    span: i64,
}

impl Level {
    pub unsafe fn new_with_node(span: i64, forward: Node) -> Self {
        Self {
            forward: Some(NonNull::new_unchecked(Box::into_raw(Box::new(forward)))),
            span,
        }
    }

    pub fn new(span: i64) -> Self {
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
    length: i64,
    /// level of node with max level
    level: usize,
}

impl SkipList {
    pub unsafe fn new() -> Self {
        let head = Node::new(String::default(),0, SKIP_LIST_MAX_LEVEL as usize);

        Self {
            head: Some(NonNull::new_unchecked(Box::into_raw(Box::new(head)))),
            tail: None,
            length: 0,
            level: 1,
        }
    }

    pub fn get_tail(&self) -> Option<NonNull<Node>> {
        self.tail
    }

    pub unsafe fn insert(&mut self, score: i64, elem: String) -> NonNull<Node> {
        let mut update = vec![NonNull::new_unchecked(Box::into_raw(Box::new(Node::default()))); SKIP_LIST_MAX_LEVEL as usize];
        let mut rank = vec![0i64; SKIP_LIST_MAX_LEVEL as usize];
        let mut x = self.head.unwrap();

        for i in (0..self.level).rev() {
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

    pub unsafe fn get_rank(&self, score: i64, elem: &String) -> i64 {
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

            if (*x.as_ptr()).elem != String::default() &&(*x.as_ptr()).score == score && sds_cmp(&(*x.as_ptr()).elem, elem) == 0 {
                return rank;
            }
        }
        0
    }

    pub unsafe fn get_elem_by_rank(&self, rank: i64) -> Option<NonNull<Node>> {
        let mut x = self.head.unwrap();
        let mut traversed = 0;
        for i in (0..self.level).rev() {
            while (*x.as_ptr()).level[i].forward.is_some() && ((*x.as_ptr()).level[i].span + traversed) <= rank {
                traversed += (*x.as_ptr()).level[i].span;
                x = (*x.as_ptr()).level[i].forward.unwrap();
            }
            if traversed == rank {
                return Some(x);
            }
        }
        None
    }

    pub unsafe fn delete(&mut self, score: i64, elem: &String) -> bool {
        let mut update = vec![NonNull::new_unchecked(Box::into_raw(Box::new(Node::default()))); SKIP_LIST_MAX_LEVEL as usize];
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

    unsafe fn delete_node(&mut self, x: NonNull<Node>, update: &Vec<NonNull<Node>>) {
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

    pub unsafe fn update_score(&mut self, cur_score: i64, elem: &str, new_score: i64) -> NonNull<Node> {
        let mut update = vec![NonNull::new_unchecked(Box::into_raw(Box::new(Node::default()))); SKIP_LIST_MAX_LEVEL as usize];
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
        return new_node;
    }
}
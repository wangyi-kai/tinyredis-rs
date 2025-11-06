use std::cmp::PartialEq;
use super::lib::{random_level, sds_cmp};
use super::SKIP_LIST_MAX_LEVEL;
use std::ptr::NonNull;
use crate::db::data_structure::dict::dict::Dict;
use std::collections::linked_list;

#[derive(Clone)]
pub struct ZSet<T> {
    pub(crate) dict: Dict<T>,
    pub(crate) zsl: SkipList,
}

#[derive(Default)]
pub struct Node {
    elem: String,
    score: f64,
    backward: Option<NonNull<Node>>,
    pub(crate) level: Vec<Level>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        let back_eq = match (self.back_ward(), other.back_ward()) {
            (Some(a), Some(b)) => a.as_ptr() == b.as_ptr(),
            (None, None) => true,
            _ => false,
        };
        self.elem == other.elem && self.score == other.score && back_eq
    }
}

impl Node {
    pub fn get_elem(&self) -> String {
        self.elem.clone()
    }

    pub fn get_score(&self) -> f64 {
        self.score
    }

    pub fn level(&self) -> &Vec<Level> {
        &self.level
    }

    pub fn back_ward(&self) -> Option<NonNull<Node>> {
        unsafe {
            self.backward.as_ref().map(|node| *node)
        }
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


pub struct Level {
    pub forward: Option<NonNull<Node>>,
    span: u64,
}

impl Clone for Level {
    fn clone(&self) -> Self {
        let node = self.forward.as_ref().map(|node| *node).unwrap();
        Self {
            forward: Some(node.clone()),
            span: self.span,
        }
    }
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

    pub fn forward(&self) -> Option<NonNull<Node>> {
        self.forward.as_ref().map(|node| *node)
    }
}

pub struct SkipList {
    /// head node
    pub head: Option<NonNull<Node>>,
    /// tail node
    pub tail: Option<NonNull<Node>>,
    /// number of nodes in skip_list
    pub length: u64,
    /// level of node with max level
    level: usize,
}

impl Clone for SkipList {
    fn clone(&self) -> Self {
        Self {
            head: Some(self.head().as_ref().unwrap().clone()),
            tail: Some(self.tail().as_ref().unwrap().clone()),
            length: self.length,
            level: self.level,
        }
    }
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

    pub(crate) fn head(&self) -> Option<NonNull<Node>> {
        self.head.as_ref().map(|node| *node)
    }

    pub(crate) fn tail(&self) -> Option<NonNull<Node>> {
        self.tail.as_ref().map(|node| *node)
    }

    #[inline(always)]
    pub fn insert(&mut self, score: f64, elem: String) -> NonNull<Node> {
        unsafe {
            let mut update = vec![
                NonNull::new_unchecked(Box::into_raw(Box::new(Node::default())));
                SKIP_LIST_MAX_LEVEL
            ];
            let mut rank = vec![0u64; SKIP_LIST_MAX_LEVEL];
            let mut x = self.head().unwrap();
            let x_ref = x.as_ref();
            assert!(!score.is_nan());

            for i in (0..self.level).rev() {
                // store rank that is crossed to reach the insert position
                rank[i] = if i == (self.level - 1) {
                    0
                } else {
                    rank[i + 1]
                };
                while x_ref.level[i].forward().is_some() &&
                    (x_ref.level[i].forward().unwrap().as_ref().get_score() < score || (x_ref.level[i].forward().unwrap().as_ref().get_score() == score && sds_cmp(&x_ref.level[i].forward().unwrap().as_ref().get_elem(), &elem) < 0)) {
                    rank[i] += x_ref.level[i].span;
                    x = x_ref.level[i].forward().unwrap();
                }
                update[i] = x.clone();
            }

            let level = random_level();
            if level > self.level {
                for i in self.level..level {
                    rank[i] = 0;
                    update[i] = self.head().unwrap();
                    update[i].as_mut().level[i].span = self.length;
                }
                self.level = level;
            }
            let mut node = Box::into_raw(Box::new(Node::new(elem, score, level)));
            for i in 0..level {
                (&mut (*node).level)[i].forward = update[i].as_ref().level[i].forward();
                update[i].as_mut().level[i].forward = Some(NonNull::new_unchecked(node));
                (&mut (*node).level)[i].span = update[i].as_ref().level[i].span - (rank[0] - rank[i]);
                update[i].as_mut().level[i].span = (rank[0] - rank[i]) + 1;
            }

            for i in level..self.level {
                update[i].as_mut().level[i].span += 1;
            }
            if update[0] == self.head().unwrap() {
                (*node).backward = None;
            } else {
                (*node).backward = Some(update[0].clone());
            }
            if (&(*node).level)[0].forward.is_some() {
                let forward = (*node).level()[0].forward().unwrap();
                (*forward.as_ptr()).backward = Some(NonNull::new_unchecked(node));
            } else {
                self.tail = Some(NonNull::new_unchecked(node));
            }
            self.length += 1;
            NonNull::new_unchecked(node)
        }
    }

    #[inline(always)]
    fn delete_node(&mut self, x: &NonNull<Node>, update: &mut Vec<NonNull<Node>>) {
        unsafe {
            for i in 0..self.level {
                if let Some(forward) = update[i].as_ref().level[i].forward() {
                    if forward == *x {
                        update[i].as_mut().level[i].span += x.as_ref().level[i].span - 1;
                        update[i].as_mut().level[i].forward = x.as_ref().level[i].forward();
                    } else {
                        update[i].as_mut().level[i].span -= 1;
                    }
                } else {
                    continue;
                }
            }

            if let Some(forward) = x.as_ref().level[0].forward() {
                (*forward.as_ptr()).backward = x.as_ref().back_ward();
            } else {
                self.tail = x.as_ref().back_ward();
            }
            let head = self.head().unwrap();
            while self.level > 1 && head.as_ref().level[self.level - 1].forward.is_none() {
                self.level -= 1;
            }
            self.length -= 1;
        }
    }

    #[inline(always)]
    pub fn delete(&mut self, score: f64, elem: &String) -> bool {
        unsafe {
            let mut update = vec![
                NonNull::new_unchecked(Box::into_raw(Box::new(Node::default())));
                SKIP_LIST_MAX_LEVEL
            ];
            let mut x = self.head().unwrap();
            let x_ref = x.as_ref();

            for i in (0..self.level).rev() {
               while x_ref.level[i].forward().is_some() &&
                    (x_ref.level[i].forward().unwrap().as_ref().get_score() < score || (x_ref.level[i].forward().unwrap().as_ref().get_score() == score && sds_cmp(&x_ref.level[i].forward().unwrap().as_ref().get_elem(), &elem) < 0)) {
                   x = x_ref.level[i].forward().unwrap();
               }
                update[i] = x.clone();
            }

            let forward = x_ref.level[0].forward();
            if forward.is_some() && score == x_ref.get_score() && sds_cmp(&x_ref.elem, elem) == 0 {
                self.delete_node(&x, &mut update);
                return true;
            }
            false
        }
    }

    #[inline(always)]
    pub fn update_score(&mut self, cur_score: f64, elem: &str, new_score: f64) -> NonNull<Node> {
        unsafe {
            let mut update = vec![
                NonNull::new_unchecked(Box::into_raw(Box::new(Node::default())));
                SKIP_LIST_MAX_LEVEL
            ];
            let mut x = self.head().unwrap();
            let x_ref = x.as_ref();

            for i in (0..self.level).rev() {
                while x_ref.level[i].forward().is_some() &&
                    (x_ref.level[i].forward().unwrap().as_ref().get_score() < cur_score || (x_ref.level[i].forward().unwrap().as_ref().get_score() == cur_score && sds_cmp(&x_ref.level[i].forward().unwrap().as_ref().get_elem(), &elem) < 0)) {
                    x = x_ref.level[i].forward().unwrap();
                }
                update[i] = x.clone();
            }

            x = x_ref.level[0].forward().unwrap();
            if x_ref.backward.is_none()
                || (x_ref.back_ward().unwrap().as_ref().score < new_score)
                && (x_ref.level[0].forward.is_none()
                    || (x_ref.level[0].forward().unwrap().as_ref().score > new_score))
            {
                x.as_mut().score = new_score;
                return x;
            }
            self.delete_node(&x, &mut update);
            let new_node = self.insert(new_score, x_ref.elem.clone());
            x.as_mut().elem = String::default();
            new_node
        }
    }

    #[inline(always)]
    pub fn get_rank(&self, score: f64, elem: &String) -> i64 {
        unsafe {
            let mut rank = 0;
            let mut x = self.head().unwrap();
            let x_ref = x.as_ref();

            for i in (0..self.level - 1).rev() {
                while x_ref.level[i].forward().is_some() &&
                    (x_ref.level[i].forward().unwrap().as_ref().get_score() < score || (x_ref.level[i].forward().unwrap().as_ref().get_score() == score && sds_cmp(&x_ref.level[i].forward().unwrap().as_ref().get_elem(), &elem) < 0)) {
                    rank += x_ref.level[i].span;
                    x = x_ref.level[i].forward().unwrap();
                }

                if x_ref.elem != String::default()
                    && x_ref.score == score
                    && sds_cmp(&x_ref.elem, elem) == 0
                {
                    return rank as i64;
                }
            }
        }
        0
    }

    #[inline(always)]
    pub fn get_elem_by_rank(&self, rank: i64) -> Option<NonNull<Node>> {
        unsafe {
            let mut x = self.head().unwrap();
            let x_ref = x.as_ref();
            let mut traversed = 0;
            for i in (0..self.level).rev() {
                while x_ref.level[i].forward.is_some()
                    && (x_ref.level[i].span + traversed) <= rank as u64
                {
                    traversed += x_ref.level[i].span;
                    x = x_ref.level[i].forward().unwrap();
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
            let mut node = (&(*self.head().unwrap().as_ptr()).level)[0].forward();
            while node.is_some() {
                //let box_node = Box::from_raw(node.unwrap().as_ptr());
                let next = (&(*node.unwrap().as_ptr()).level)[0].forward();
                node = next;
            }
        }
    }
}

use std::marker::PhantomData;
use std::ptr::NonNull;

#[derive(PartialEq)]
pub struct Node<T> {
    value: T,
    prev: Option<NonNull<Node<T>>>,
    next: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    pub fn prev_node(&self) -> Option<NonNull<Node<T>>> {
        self.prev
    }

    pub fn next_node(&self) -> Option<NonNull<Node<T>>> {
        self.next
    }

    pub fn prev_value(&self) -> Option<&T> {
        unsafe {
            self.prev.map(|node| &(*node.as_ptr()).value)
        }
    }

    pub fn next_value(&self) -> Option<&T> {
        unsafe {
            self.next.map(|node| &(*node.as_ptr()).value)
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}

pub struct List<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
}

impl<T: PartialEq> List<T> {
    pub fn create() -> List<T> {
        Self {
            head: None,
            tail: None,
            len: 0,
        }
    }

     pub fn add_node_head(&mut self, elem: T) {
         unsafe {
             let new_node = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                 value: elem,
                 prev: None,
                 next: None,
             })));
             if self.len == 0 {
                 self.head = Some(new_node);
                 self.tail = Some(new_node);
             } else {
                 (*new_node.as_ptr()).prev = None;
                 (*new_node.as_ptr()).next = self.head;
                 (*self.head.unwrap().as_ptr()).prev = Some(new_node);
                 self.head = Some(new_node);
             }
             self.len += 1;
         }
    }

    pub fn add_node_tail(&mut self, elem: T) {
        unsafe {
            let new_node = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                value: elem,
                prev: None,
                next: None,
            })));

            if self.len == 0 {
                self.head = Some(new_node);
                self.tail = Some(new_node);
            } else {
                (*new_node.as_ptr()).prev = self.tail;
                (*new_node.as_ptr()).next = None;
                (*self.tail.unwrap().as_ptr()).next = Some(new_node);
                self.tail = Some(new_node);
            }
            self.len += 1;
        }
    }

    pub fn delete(&mut self, elem: T) {
        unsafe {
            if self.len > 0 {
                let mut cur = self.head;
                while let Some(node) = cur {
                    if (*node.as_ptr()).value == elem {
                        self.delete_node(node);
                    }
                    cur = (*node.as_ptr()).next;
                }
            }
        }
    }

    pub fn delete_node(&mut self, node: NonNull<Node<T>>) {
        unsafe {
            if (*node.as_ptr()).prev.is_some() {
                (*(*node.as_ptr()).prev.unwrap().as_ptr()).next = (*node.as_ptr()).next;
            } else {
                self.head = (*node.as_ptr()).next;
            }

            if (*node.as_ptr()).next.is_some() {
                (*(*node.as_ptr()).next.unwrap().as_ptr()).prev = (*node.as_ptr()).prev;
            } else {
                self.tail = (*node.as_ptr()).prev;
            }
            (*node.as_ptr()).prev = None;
            (*node.as_ptr()).next = None;
            self.len -= 1;
        }
    }

    pub fn index(&self, mut index: i64) -> Option<NonNull<Node<T>>> {
        unsafe {
            if index < 0 {
                index = (-index) - 1;
                let mut cur = self.tail;
                while let Some(node) = cur {
                    if index == 0 {
                        break;
                    }
                    cur = (*node.as_ptr()).prev;
                    index -= 1;
                }
                cur
            } else {
                let mut cur = self.head;
                while let Some(node) = cur {
                    if index == 0 {
                        break;
                    }
                    cur = (*node.as_ptr()).next;
                    index -= 1;
                }
                cur
            }
        }
    }

    pub fn length(&self) -> usize {
        self.len
    }

    pub fn search_key(&self, key: T) -> Option<NonNull<Node<T>>> {
        None
    }

    pub fn empty(&mut self) {
        unsafe {
            let mut current = self.head;
            while self.len > 0 {
                let box_node = Box::from_raw(current.unwrap().as_ptr());
                current = box_node.next;
                self.len -= 1;
            }
            self.head = None;
            self.tail = None;
        }
    }
}

pub struct Iter<'a, T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    _bool: PhantomData<&'a T>,
}

impl<T> List<T> {
    pub fn iter(&self) -> Iter<T> {
        Iter {
            head: self.head,
            tail: self.tail,
            len: self.len,
            _bool: PhantomData,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len > 0 {
            self.head.map(|node| unsafe {
                self.len -= 1;
                self.head = (*node.as_ptr()).next;
                &(*node.as_ptr()).value
            })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len > 0 {
            self.tail.map(|node| unsafe {
                self.len -= 1;
                self.tail = (*node.as_ptr()).prev;
                &(*node.as_ptr()).value
            })
        } else {
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}




use std::ptr::NonNull;
use crate::data_structure::adlist::adlist::{Node};
use crate::kvstore::kvstore::KvStoreDictMetadata;

pub struct KvStoreDictMetaBase<T> {
    pub rehashing_node: Option<NonNull<Node<T>>>,
}

pub struct KvStoreDictMetaEx<T> {
    pub base: KvStoreDictMetaBase<T>,
    pub mata: KvStoreDictMetadata,
}


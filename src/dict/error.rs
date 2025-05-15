use std::fmt::Debug;

#[derive(thiserror::Error, Debug, Clone)]
pub enum HashError {
    #[error("[Dict]Hash Table Insert Fail")]
    DictInsertError,
    #[error("[Dict]Dict Key Has Exist")]
    DictEntryDup,
    #[error("[Dict]Dict Is Empty")]
    DictEmpty,
    #[error("[Dict]Key: {0} Is Not Exist")]
    DictNoKey(String),
    #[error("[Dict]Rehash Error: {0}!")]
    RehashErr(String),
    #[error("[Dict]Expand Error: {0}!")]
    ExpandErr(String),
    #[error("[Dict]Dict Is Rehashing")]
    IsRehashing,
    #[error("[Dict]Shrink error({0})")]
    ShrinkErr(i64),
}
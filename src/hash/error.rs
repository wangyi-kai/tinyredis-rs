use std::fmt::Debug;

#[derive(thiserror::Error, Debug, Clone)]
pub enum HashError {
    #[error("Hash Table Insert Fail")]
    DictInsertError,
    #[error("Dict Key Has Exist")]
    DictEntryDup,
    #[error("Dict Is Empty")]
    DictEmpty,
    #[error("Key: {0} Is Not Exist")]
    DictNoKey(String),
    #[error("Rehash Error: {0}!")]
    RehashErr(String),
    #[error("Expand Error: {0}!")]
    ExpandErr(String),
    #[error("Dict Is Rehashing")]
    IsRehashing,
}
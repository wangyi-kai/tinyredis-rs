use std::fmt::Debug;

#[derive(thiserror::Error, Debug, Clone)]
pub enum HashError {
    #[error("Hash Table Insert Fail")]
    DictInsertError,
    #[error("Dict Key Has Exist")]
    DictEntryDup,
    #[error("Dict Is Empty")]
    DictEmpty,
    #[error("Hash Key Is Not Exist")]
    DictNoKey,
    #[error("Rehash Error!")]
    RehashErr,
}
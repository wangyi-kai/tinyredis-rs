use std::io;


#[derive(thiserror::Error, Debug)]
pub enum PersistError {
    #[error("[RDB] open file error({0})")]
    FileError(i64),
    #[error("[RDB] {0}")]
    LoadErr(String),
    #[error("[RDB] {0}")]
    EncodingErr(String),
    #[error("[RDB] {0}")]
    TypeErr(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
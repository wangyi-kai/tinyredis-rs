use thiserror::Error;

#[derive(thiserror::Error, Debug)]
pub enum PersistError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("[RDB] open file error({0})")]
    FileError(i64),
    #[error("[RDB] {0}")]
    LoadErr(String),
    #[error("[RDB] {0}")]
    EncodingErr(String),
    #[error("[RDB] {0}")]
    TypeErr(String),
}
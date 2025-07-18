
#[derive(thiserror::Error, Debug, Clone)]
pub enum CommandError {
    #[error("[Parser]Parse from frame error({0})")]
    ParseError(i64),
    #[error("Object Type Mismatch({0})")]
    ObjectTypeError(i64),
    #[error("Not Support Command({0})")]
    NotSupport(String),
    #[error("{0} args error")]
    ArgsErr(String),
}

unsafe impl Send for CommandError {}
unsafe impl Sync for CommandError {}
use crate::persistence::error::PersistError;


#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("receive error({0})")]
    ReceiveErr(i64),
    #[error(transparent)]
    PersistErr(#[from] PersistError),
}
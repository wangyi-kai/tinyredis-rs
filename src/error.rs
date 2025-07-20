
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("receive error({0})")]
    ReceiveErr(i64),
}
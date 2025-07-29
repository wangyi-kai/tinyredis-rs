#[derive(thiserror::Error, Debug, Clone)]
pub enum ZipListError {
    #[error("Position out of range(0)")]
    OutOfRange(usize),
    #[error("Invalid length size")]
    InValidLenSize,
    #[error("A string of zero length or excessive length")]
    InValidString,
    #[error("FirstDigitError")]
    InvalidFirstDigit,
    #[error("InvalidChar")]
    InvalidChar,
    #[error("Mul overflow")]
    OverFlowMul,
    #[error("Add overflow")]
    OverFlowAdd,
    #[error("Negative overflow")]
    OverFlowNegative,
    #[error("Positive overflow")]
    OverFlowPositive,
    #[error("{0} not found")]
    NotFund(String),
}

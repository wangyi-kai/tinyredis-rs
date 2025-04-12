
pub enum ZipListError {
    #[error("Position out of range(0)")]
    OutOfRange(usize),
    #[error("Invalid length size")]
    InValidLenSize,
    #[error("A string of zero length or excessive length")]
    InValidString,
    #[error("FirstDigitError")]
    InvalidFirstDigit,
}
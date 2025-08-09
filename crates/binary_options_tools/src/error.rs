use crate::pocketoption::error::PocketError;

#[derive(thiserror::Error, Debug)]
pub enum BinaryOptionsError {
    #[error("Pocket Options Error: {0}")]
    PocketOptions(Box<PocketError>),

    /// Couldn't parse f64 to Decimal
    #[error("Couldn't parse f64 to Decimal: {0}")]
    ParseFloat(String),
    /// Couldn't parse Decimal to f64
    #[error("Couldn't parse Decimal to f64: {0}")]
    ParseDecimal(String),
}

pub type BinaryOptionsResult<T> = Result<T, BinaryOptionsError>;

impl From<PocketError> for BinaryOptionsError {
    fn from(error: PocketError) -> Self {
        BinaryOptionsError::PocketOptions(Box::new(error))
    }
}

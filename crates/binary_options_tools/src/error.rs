use crate::pocketoption::error::PocketError;
use binary_options_tools_core::error::CoreError;
use rust_decimal::Decimal;
use std::num::ParseFloatError;

#[derive(thiserror::Error, Debug)]
pub enum BinaryOptionsError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("Pocket Options Error: {0}")]
    PocketOptions(#[from] PocketError),

    /// Couldn't parse f64 to Decimal
    #[error("Couldn't parse f64 to Decimal: {0}")]
    ParseFloat(#[from] ParseFloatError),

    /// Couldn't parse Decimal to f64
    #[error("Couldn't parse Decimal to f64: {0}")]
    ParseDecimal(String),

    /// General error with a message
    #[error("General error: {0}")]
    General(String),
}

pub type BinaryOptionsResult<T> = Result<T, BinaryOptionsError>;

impl From<Decimal> for BinaryOptionsError {
    fn from(decimal: Decimal) -> Self {
        BinaryOptionsError::ParseDecimal(format!("Failed to convert Decimal {} to f64", decimal))
    }
}

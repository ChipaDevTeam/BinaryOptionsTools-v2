use binary_options_tools::{error::BinaryOptionsToolsError, pocketoption_pre::error::PocketError};
use pyo3::{exceptions::PyValueError, PyErr};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum BinaryErrorPy {
    #[error("BinaryOptionsError, {0}")]
    BinaryOptionsError(#[from] BinaryOptionsToolsError),
    #[error("PocketOptionError, {0}")]
    PocketOptionError(#[from] PocketError),

    #[error("Uninitialized, {0}")]
    Uninitialized(String),
    #[error("Error descerializing data, {0}")]
    DeserializingError(#[from] serde_json::Error),
    #[error("UUID parsing error, {0}")]
    UuidParsingError(#[from] uuid::Error),
    #[error("Trade not found, haven't found trade for id '{0}'")]
    TradeNotFound(Uuid),
    #[error("Operation not allowed")]
    NotAllowed(String),
    #[error("Invalid Regex pattern, {0}")]
    InvalidRegexError(#[from] regex::Error),
}

impl From<BinaryErrorPy> for PyErr {
    fn from(value: BinaryErrorPy) -> Self {
        PyValueError::new_err(value.to_string())
    }
}

pub type BinaryResultPy<T> = Result<T, BinaryErrorPy>;

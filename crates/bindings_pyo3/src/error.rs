use binary_options_tools::{error::BinaryOptionsError, pocketoption::error::PocketError};
use pyo3::{exceptions::PyValueError, PyErr};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum BinaryErrorPy {
    #[error("BinaryOptionsError, {0}")]
    BinaryOptionsError(Box<BinaryOptionsError>),
    #[error("PocketOptionError, {0}")]
    PocketOptionError(Box<PocketError>),

    #[error("Uninitialized, {0}")]
    Uninitialized(String),
    #[error("Error deserializing data, {0}")]
    DeserializingError(#[from] serde_json::Error),
    #[error("UUID parsing error, {0}")]
    UuidParsingError(#[from] uuid::Error),
    #[error("Trade not found, haven't found trade for id '{0}'")]
    TradeNotFound(Uuid),
    #[error("Operation not allowed: {0}")]
    NotAllowed(String),
    #[error("Invalid Regex pattern, {0}")]
    InvalidRegexError(#[from] regex::Error),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

pyo3::create_exception!(BinaryOptionsToolsV2, PocketOptionError, pyo3::exceptions::PyException);
pyo3::create_exception!(BinaryOptionsToolsV2, TradeNotFoundError, pyo3::exceptions::PyException);
pyo3::create_exception!(BinaryOptionsToolsV2, UninitializedError, pyo3::exceptions::PyException);
pyo3::create_exception!(BinaryOptionsToolsV2, NotAllowedError, pyo3::exceptions::PyException);
pyo3::create_exception!(BinaryOptionsToolsV2, InvalidParameterError, pyo3::exceptions::PyException);

impl From<BinaryErrorPy> for PyErr {
    fn from(value: BinaryErrorPy) -> Self {
        match value {
            BinaryErrorPy::PocketOptionError(..) => PocketOptionError::new_err(value.to_string()),
            BinaryErrorPy::TradeNotFound(..) => TradeNotFoundError::new_err(value.to_string()),
            BinaryErrorPy::Uninitialized(..) => UninitializedError::new_err(value.to_string()),
            BinaryErrorPy::NotAllowed(..) => NotAllowedError::new_err(value.to_string()),
            BinaryErrorPy::InvalidParameter(..) => InvalidParameterError::new_err(value.to_string()),
            _ => PyValueError::new_err(value.to_string()),
        }
    }
}

pub type BinaryResultPy<T> = Result<T, BinaryErrorPy>;

impl From<BinaryOptionsError> for BinaryErrorPy {
    fn from(value: BinaryOptionsError) -> Self {
        BinaryErrorPy::BinaryOptionsError(Box::new(value))
    }
}

impl From<PocketError> for BinaryErrorPy {
    fn from(value: PocketError) -> Self {
        BinaryErrorPy::PocketOptionError(Box::new(value))
    }
}

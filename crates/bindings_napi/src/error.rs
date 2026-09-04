use binary_options_tools::error::BinaryOptionsError;
use binary_options_tools::pocketoption::error::PocketError;
use napi::{Error as NapiError, Status};
use thiserror::Error;
use uuid::Uuid;

/// Errors surfaced by the Node.js bindings.
///
/// Every variant is turned into a JavaScript `Error` whose message is prefixed
/// with the variant name, so callers can branch on `error.name` (the JS wrapper
/// promotes the prefix into `name`) or simply print the message.
#[derive(Error, Debug)]
pub enum BinaryErrorNode {
    #[error("BinaryOptionsError: {0}")]
    BinaryOptions(Box<BinaryOptionsError>),
    #[error("PocketOptionError: {0}")]
    PocketOption(Box<PocketError>),
    #[error("UninitializedError: {0}")]
    Uninitialized(String),
    #[error("DeserializingError: {0}")]
    Deserializing(#[from] serde_json::Error),
    #[error("UuidParsingError: {0}")]
    UuidParsing(#[from] uuid::Error),
    #[error("TradeNotFoundError: haven't found trade for id '{0}'")]
    TradeNotFound(Uuid),
    #[error("NotAllowedError: {0}")]
    NotAllowed(String),
    #[error("InvalidRegexError: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("InvalidParameterError: {0}")]
    InvalidParameter(String),
    #[error("TimeoutError: {0}")]
    Timeout(String),
}

impl From<BinaryOptionsError> for BinaryErrorNode {
    fn from(value: BinaryOptionsError) -> Self {
        Self::BinaryOptions(Box::new(value))
    }
}

impl From<PocketError> for BinaryErrorNode {
    fn from(value: PocketError) -> Self {
        Self::PocketOption(Box::new(value))
    }
}

impl From<BinaryErrorNode> for NapiError {
    fn from(value: BinaryErrorNode) -> Self {
        NapiError::new(Status::GenericFailure, value.to_string())
    }
}

/// Shorthand used by the binding methods, which all return `napi::Result`.
pub fn napi_err(error: impl Into<BinaryErrorNode>) -> NapiError {
    error.into().into()
}

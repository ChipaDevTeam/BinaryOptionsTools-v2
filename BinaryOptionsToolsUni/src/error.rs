use binary_options_tools::error::BinaryOptionsError;
use binary_options_tools::pocketoption::error::PocketError;
use thiserror::Error;

#[derive(Error, Debug, uniffi::Error)]
pub enum UniError {
    #[error("An error occurred in the underlying binary_options_tools crate: {0}")]
    BinaryOptions(String),
    #[error("An error occurred in the PocketOption client: {0}")]
    PocketOption(String),
    #[error("An error occurred with UUID parsing: {0}")]
    Uuid(String),
    #[error("An error occurred with validator: {0}")]
    Validator(String),
    #[error("General error: {0}")]
    General(String),
}

impl From<BinaryOptionsError> for UniError {
    fn from(e: BinaryOptionsError) -> Self {
        match e {
            BinaryOptionsError::PocketOptions(pocket_error) => {
                UniError::PocketOption(pocket_error.to_string())
            }
            _ => UniError::BinaryOptions(e.to_string()),
        }
    }
}

impl From<PocketError> for UniError {
    fn from(e: PocketError) -> Self {
        UniError::PocketOption(e.to_string())
    }
}

use thiserror::Error;

#[derive(Error, Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum UniError {
    #[error("An error occurred in the underlying binary_options_tools crate: {message}")]
    Internal { message: String },
}

impl From<binary_options_tools::error::BinaryOptionsError> for UniError {
    fn from(e: binary_options_tools::error::BinaryOptionsError) -> Self {
        UniError::Internal {
            message: e.to_string(),
        }
    }
}

use crate::pocketoption::error::PocketError;

#[derive(thiserror::Error, Debug)]
pub enum BinaryOptionsError {
    #[error("Pocket Options Error: {0}")]
    PocketOptions(Box<PocketError>),
}

pub type BinaryOptionsResult<T> = Result<T, BinaryOptionsError>;

impl From<PocketError> for BinaryOptionsError {
    fn from(error: PocketError) -> Self {
        BinaryOptionsError::PocketOptions(Box::new(error))
    }
}
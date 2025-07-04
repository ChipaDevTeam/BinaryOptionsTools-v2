use binary_options_tools_core_pre::error::CoreError;


#[derive(thiserror::Error, Debug)]
pub enum PocketError {
    #[error("Failed to join task: {0}")]
    Core(#[from] Box<CoreError>),
    #[error("State builder error, {0}")]
    StateBuilder(String),
}

pub type PocketResult<T> = Result<T, PocketError>;

impl From<CoreError> for PocketError {
    fn from(err: CoreError) -> Self {
        PocketError::Core(Box::new(err))
    }
}
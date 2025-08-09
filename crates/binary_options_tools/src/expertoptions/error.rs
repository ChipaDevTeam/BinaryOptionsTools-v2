use binary_options_tools_core_pre::error::CoreError;

#[derive(thiserror::Error, Debug)]
pub enum ExpertOptionsError {
    #[error("Serde JSON deserialization error: {0}")]
    Deserializing(#[from] serde_json::Error),

    #[error("Serde JSON serialization error: {0}")]
    Serializing(serde_json::Error),

    #[error("Failed to join task: {0}")]
    Core(#[from] Box<CoreError>),
}

pub type ExpertOptionsResult<T> = Result<T, ExpertOptionsError>;

impl From<CoreError> for ExpertOptionsError {
    fn from(err: CoreError) -> Self {
        ExpertOptionsError::Core(Box::new(err))
    }
}

use crate::error::{BinaryOptionsResult, BinaryOptionsToolsError};

use super::traits::{MessageTransfer, ValidatorTrait};

pub fn validate<Transfer>(
    validator: &(dyn ValidatorTrait<Transfer> + Send + Sync),
    message: Transfer,
) -> BinaryOptionsResult<Option<Transfer>>
where
    Transfer: MessageTransfer,
{
    if let Some(e) = message.error() {
        Err(BinaryOptionsToolsError::WebSocketMessageError(
            e.to_string(),
        ))
    } else if validator.validate(&message) {
        Ok(Some(message))
    } else {
        Ok(None)
    }
}

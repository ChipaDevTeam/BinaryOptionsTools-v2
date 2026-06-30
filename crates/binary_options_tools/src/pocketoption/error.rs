use std::time::Duration;

use binary_options_tools_core::error::CoreError;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::error::BinaryOptionsError;
use crate::pocketoption::modules::subscriptions::SubscriptionError;

#[derive(thiserror::Error, Debug)]
pub enum PocketError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("State builder error: {0}")]
    StateBuilder(String),
    #[error("Invalid asset: {0}")]
    InvalidAsset(String),

    /// Error opening order.
    #[error("Failed to open order: {error}, amount: {amount}, asset: {asset}")]
    FailOpenOrder {
        error: String,
        amount: Decimal,
        asset: String,
    },

    /// Error finding deal.
    #[error("Failed to find deal: {0}")]
    DealNotFound(Uuid),

    /// Timeout error.
    #[error("Timeout error: {task} in {context} after {duration:?}")]
    Timeout {
        task: String, // The task that timed out, eg "check-results",
        context: String,
        duration: Duration,
    },

    #[error("Invalid period: {0}")]
    InvalidPeriod(u32),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Module {module_name} stopped: {context}")]
    ModuleStopped {
        module_name: String,
        context: String,
    },

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("General error: {0}")]
    General(String),

    #[error("Subscription error: {0}")]
    Subscription(#[from] SubscriptionError),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

pub type PocketResult<T> = Result<T, PocketError>;

impl From<BinaryOptionsError> for PocketError {
    fn from(error: BinaryOptionsError) -> Self {
        match error {
            BinaryOptionsError::PocketOptions(pocket_error) => pocket_error,
            _ => PocketError::General(format!("BinaryOptionsError: {:?}", error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_pocket_error_invalid_asset() {
        let err = PocketError::InvalidAsset("BTC/USD".to_string());
        assert_eq!(err.to_string(), "Invalid asset: BTC/USD");
    }

    #[test]
    fn test_pocket_error_fail_open_order() {
        let err = PocketError::FailOpenOrder {
            error: "insufficient funds".to_string(),
            amount: Decimal::new(100, 0),
            asset: "EUR/USD".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("insufficient funds"));
        assert!(msg.contains("100"));
        assert!(msg.contains("EUR/USD"));
    }

    #[test]
    fn test_pocket_error_deal_not_found() {
        let id = Uuid::new_v4();
        let err = PocketError::DealNotFound(id);
        assert!(err.to_string().contains(&id.to_string()));
    }

    #[test]
    fn test_pocket_error_timeout() {
        let err = PocketError::Timeout {
            task: "check-results".to_string(),
            context: "polling".to_string(),
            duration: Duration::from_secs(30),
        };
        let msg = err.to_string();
        assert!(msg.contains("check-results"));
        assert!(msg.contains("polling"));
        assert!(msg.contains("30"));
    }

    #[test]
    fn test_pocket_error_general() {
        let err = PocketError::General("something went wrong".to_string());
        assert_eq!(err.to_string(), "General error: something went wrong");
    }

    #[test]
    fn test_pocket_error_configuration() {
        let err = PocketError::Configuration("missing config".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing config");
    }

    #[test]
    fn test_pocket_result_type_alias() {
        let ok: PocketResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
    }
}

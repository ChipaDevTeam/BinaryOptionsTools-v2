use binary_options_tools_core::error::CoreError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CloseOptionError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("State builder error: {0}")]
    StateBuilder(String),

    #[error("Invalid asset: {0}")]
    InvalidAsset(String),

    #[error("Failed to open order: {error}, amount: {amount}, asset: {asset}")]
    FailOpenOrder {
        error: String,
        amount: f64,
        asset: String,
    },

    #[error("Failed to find deal: {0}")]
    DealNotFound(String),

    #[error("Timeout error: {task} in {context} after {duration:?}")]
    Timeout {
        task: String,
        context: String,
        duration: std::time::Duration,
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

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("General error: {0}")]
    General(String),

    #[error("Custom error: {0}")]
    Custom(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Socket.IO error: {0}")]
    SocketIo(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("CloseOption API error: {head} (code: {code}, pair: {pair})")]
    ApiError {
        head: String,
        code: String,
        pair: String,
    },
}

pub type CloseOptionResult<T> = Result<T, CloseOptionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CloseOptionError::InvalidAsset("EURUSD".to_string());
        assert_eq!(err.to_string(), "Invalid asset: EURUSD");
    }

    #[test]
    fn test_fail_open_order_error() {
        let err = CloseOptionError::FailOpenOrder {
            error: "timeout".to_string(),
            amount: 100.0,
            asset: "EURUSD".to_string(),
        };
        assert!(err.to_string().contains("Failed to open order"));
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("EURUSD"));
    }
}

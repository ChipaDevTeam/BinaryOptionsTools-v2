//! # Binary Options Tools
//!
//! A comprehensive library for binary options trading tools and utilities.
//!
//! This crate provides modules for interacting with various binary options platforms,
//! error handling utilities, and streaming capabilities for real-time data processing.
//!
//! ## Modules
//!
//! - `pocketoption` - Integration with PocketOption platform
//! - `expertoptions` - Integration with ExpertOption platform  
//! - `reimports` - Common re-exports for convenience
//! - `error` - Error handling types and utilities
//! - `stream` - Streaming utilities including receiver streams and logging layers
//!
//! ## Features
//!
//! - Asynchronous operations with tokio support
//! - Serialization/deserialization with serde
//! - Structured logging with tracing
//! - Timeout handling with custom macros
//! - Stream processing capabilities
//!
//! // Use the streaming utilities for real-time data processing
//! // Serialize and deserialize data with the provided macros
//! // Apply timeouts to async operations
//! ```
pub mod expertoptions;
pub mod pocketoption;

pub mod reimports;
pub mod traits;
pub mod utils;
pub mod validator;

pub mod error;
pub mod stream {
    pub use binary_options_tools_core_pre::reimports::*;
    pub use binary_options_tools_core_pre::utils::stream::RecieverStream;
    pub use binary_options_tools_core_pre::utils::tracing::stream_logs_layer;
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde::{Deserialize, Serialize};
    use tokio::time::sleep;
    use tracing::debug;

    use binary_options_tools_macros::{deserialize, serialize, timeout};
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct Test {
        name: String,
    }
    use tracing_test::traced_test; // Add this import.

    #[test]
    fn test_deserialize_macro() {
        let test = Test {
            name: "Test".to_string(),
        };
        let test_str = serialize!(&test).unwrap();
        let test2 = deserialize!(Test, &test_str).unwrap();
        assert_eq!(test, test2)
    }

    struct Tester;

    #[tokio::test]
    #[traced_test]
    async fn test_timeout_macro() -> anyhow::Result<()> {
        #[timeout(1, tracing(level = "info", skip(_tester)))]
        async fn this_is_a_test(_tester: Tester) -> anyhow::Result<()> {
            debug!("Test");
            sleep(Duration::from_secs(0)).await;
            Ok(())
        }

        this_is_a_test(Tester).await
    }
}

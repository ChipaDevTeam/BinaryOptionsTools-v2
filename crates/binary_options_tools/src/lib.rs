pub mod pocketoption;
pub mod pocketoption_pre;
pub mod reimports;

pub mod stream {
    pub use binary_options_tools_core::general::stream::RecieverStream;
    pub use binary_options_tools_core::utils::tracing::stream_logs_layer;
}

pub mod error {
    pub use binary_options_tools_core::error::{BinaryOptionsResult, BinaryOptionsToolsError};
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde::{Deserialize, Serialize};
    use tokio::time::sleep;
    use tracing::debug;

    use binary_options_tools_core::utils::tracing::start_tracing;
    use binary_options_tools_macros::{deserialize, serialize, timeout};
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct Test {
        name: String,
    }

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
    async fn test_timeout_macro() -> anyhow::Result<()> {
        start_tracing(true).unwrap();

        #[timeout(1, tracing(level = "info", skip(_tester)))]
        async fn this_is_a_test(_tester: Tester) -> anyhow::Result<()> {
            debug!("Test");
            sleep(Duration::from_secs(0)).await;
            Ok(())
        }

        this_is_a_test(Tester).await
    }
}

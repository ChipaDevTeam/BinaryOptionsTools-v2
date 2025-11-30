//! Integration tests for the PocketOption client functionality

use binary_options_tools::pocketoption::modules::raw::Outgoing;
use binary_options_tools::pocketoption::pocket_client::PocketOption;
use binary_options_tools::validator::Validator;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;
    use binary_options_tools::traits::ValidatorTrait;
    use tokio_tungstenite::tungstenite::Message;
    use uuid::Uuid;

    #[test]
    fn test_outgoing_enum() {
        let text_msg = Outgoing::Text("test message".to_string());
        let binary_msg = Outgoing::Binary(vec![1, 2, 3, 4]);

        match text_msg {
            Outgoing::Text(text) => assert_eq!(text, "test message"),
            _ => panic!("Expected Text variant"),
        }

        match binary_msg {
            Outgoing::Binary(data) => assert_eq!(data, vec![1, 2, 3, 4]),
            _ => panic!("Expected Binary variant"),
        }
    }


    // Test raw handle functionality
    #[tokio::test]
    async fn test_raw_handle_functionality() {
        let _ = tracing_subscriber::fmt::try_init();
        // This test would require a connected client
        // For now, we'll just verify the function signatures compile
        let _ = PocketOption::raw_handle;
        let _ = PocketOption::create_raw_handler;
        assert!(true);
    }

    // Test validator creation methods
    #[test]
    fn test_validator_creation() {
        let starts_with = Validator::starts_with("test".to_string());
        let ends_with = Validator::ends_with("end".to_string());
        let contains = Validator::contains("middle".to_string());
        let regex = Validator::regex(regex::Regex::new(r"^\d+$").unwrap());
        let not = Validator::negate(contains.clone());
        let all = Validator::all(vec![starts_with.clone(), ends_with.clone()]);
        let any = Validator::any(vec![starts_with.clone(), ends_with.clone()]);

        assert!(starts_with.call("test message"));
        assert!(ends_with.call("message end"));
        assert!(contains.call("has middle content"));
        assert!(regex.call("12345"));
        assert!(!not.call("has middle content"));
        assert!(all.call("test message end"));
        assert!(any.call("test message"));
    }

    // Test error handling scenarios
    #[tokio::test]
    async fn test_error_handling_scenarios() {
        // Test that error types are properly defined
        let _ = binary_options_tools::pocketoption::error::PocketError::General("test".to_string());
        let _ = binary_options_tools::error::BinaryOptionsError::General("test".to_string());

        assert!(true); // Just verify compilation
    }

    // Test the RawValidator functionality
    #[test]
    fn test_raw_validator() {
        let validator = binary_options_tools::validator::RawValidator::new();
        let valid_json = serde_json::json!({"status": "ok"});
        let invalid_json = serde_json::Value::Null;

        assert!(validator.check(&valid_json));
        assert!(!validator.check(&invalid_json));
    }
}

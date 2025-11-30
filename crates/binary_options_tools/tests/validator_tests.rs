//! Unit tests for the Validator implementation

use binary_options_tools::validator::{RawValidator, Validator};
use regex::Regex;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use binary_options_tools::traits::ValidatorTrait;

    #[test]
    fn test_validator_none() {
        let validator = Validator::None;
        assert!(validator.call("any string"));
        assert!(validator.call(""));
        assert!(validator.call("Hello World"));
    }

    #[test]
    fn test_validator_starts_with() {
        let validator = Validator::starts_with("Hello".to_string());
        assert!(validator.call("Hello World"));
        assert!(validator.call("Hello"));
        assert!(!validator.call("hello World"));
        assert!(!validator.call("Hi Hello"));
        assert!(!validator.call(""));
    }

    #[test]
    fn test_validator_ends_with() {
        let validator = Validator::ends_with("World".to_string());
        assert!(validator.call("Hello World"));
        assert!(validator.call("World"));
        assert!(!validator.call("Hello world"));
        assert!(!validator.call("World Hello"));
        assert!(!validator.call(""));
    }

    #[test]
    fn test_validator_contains() {
        let validator = Validator::contains("World".to_string());
        assert!(validator.call("Hello World"));
        assert!(validator.call("World"));
        assert!(validator.call("Say World to me"));
        assert!(!validator.call("Hello world"));
        assert!(!validator.call("Wor ld"));
        assert!(!validator.call(""));
    }

    #[test]
    fn test_validator_regex() {
        let regex = Regex::new(r"^[A-Z][a-z]+$").unwrap();
        let validator = Validator::regex(regex);
        assert!(validator.call("Hello"));
        assert!(validator.call("World"));
        assert!(!validator.call("hello"));
        assert!(!validator.call("HELLO"));
        assert!(!validator.call("Hello123"));
        assert!(!validator.call(""));
    }

    #[test]
    fn test_validator_negate() {
        let base_validator = Validator::contains("error".to_string());
        let validator = Validator::negate(base_validator);
        assert!(!validator.call("An error occurred"));
        assert!(validator.call("Success message"));
        assert!(validator.call(""));
    }

    #[test]
    fn test_validator_all() {
        let v1 = Validator::starts_with("Hello".to_string());
        let v2 = Validator::contains("World".to_string());
        let validator = Validator::all(vec![v1, v2]);
        assert!(validator.call("Hello World"));
        assert!(validator.call("Hello Beautiful World"));
        assert!(!validator.call("Hello"));
        assert!(!validator.call("World"));
        assert!(!validator.call("Hi World"));
    }

    #[test]
    fn test_validator_any() {
        let v1 = Validator::starts_with("Hello".to_string());
        let v2 = Validator::ends_with("World".to_string());
        let validator = Validator::any(vec![v1, v2]);
        assert!(validator.call("Hello there"));
        assert!(validator.call("Hi World"));
        assert!(validator.call("Hello World"));
        assert!(!validator.call("Hi there"));
        assert!(!validator.call(""));
    }

    #[test]
    fn test_validator_add() {
        // Test adding to All validator
        let mut validator = Validator::all(vec![Validator::starts_with("Hello".to_string())]);
        validator.add(Validator::contains("World".to_string()));
        assert!(validator.call("Hello World"));
        assert!(!validator.call("Hello"));

        // Test adding to Any validator
        let mut validator = Validator::any(vec![Validator::starts_with("Hello".to_string())]);
        validator.add(Validator::ends_with("World".to_string()));
        assert!(validator.call("Hello there"));
        assert!(validator.call("Hi World"));

        // Test adding to single validator
        let mut validator = Validator::starts_with("Hello".to_string());
        validator.add(Validator::contains("World".to_string()));
        assert!(validator.call("Hello World"));
        assert!(!validator.call("Hello"));
        assert!(!validator.call("Hi World"));
    }

    #[test]
    fn test_raw_validator() {
        let validator = RawValidator::new();
        assert!(validator.check(&serde_json::json!("test")));
        assert!(validator.check(&serde_json::json!({"key": "value"})));
        assert!(!validator.check(&serde_json::Value::Null));
    }

    // Test custom validator implementation
    struct CustomValidator;

    impl binary_options_tools::traits::ValidatorTrait for CustomValidator {
        fn call(&self, data: &str) -> bool {
            data.len() > 5
        }
    }

    #[test]
    fn test_validator_custom() {
        let custom_validator = Arc::new(CustomValidator);
        let validator = Validator::custom(custom_validator);
        assert!(validator.call("123456"));
        assert!(!validator.call("12345"));
        assert!(!validator.call(""));
    }
}

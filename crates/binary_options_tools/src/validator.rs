use std::fmt;
use std::sync::Arc;

use regex::Regex;
use serde_json::Value;

use crate::traits::ValidatorTrait;

#[derive(Clone, Default)]
pub enum Validator {
    #[default]
    None,
    StartsWith(String),
    EndsWith(String),
    Contains(String),
    Regex(Regex),
    Not(Box<Validator>),
    All(Box<Vec<Validator>>),
    Any(Box<Vec<Validator>>),
    Custom(Arc<dyn ValidatorTrait + Send + Sync>),
}

impl fmt::Debug for Validator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Validator::None => write!(f, "Validator::None"),
            Validator::StartsWith(s) => f.debug_tuple("Validator::StartsWith").field(s).finish(),
            Validator::EndsWith(s) => f.debug_tuple("Validator::EndsWith").field(s).finish(),
            Validator::Contains(s) => f.debug_tuple("Validator::Contains").field(s).finish(),
            Validator::Regex(r) => f.debug_tuple("Validator::Regex").field(r).finish(),
            Validator::Not(v) => f.debug_tuple("Validator::Not").field(v).finish(),
            Validator::All(v) => f.debug_tuple("Validator::All").field(v).finish(),
            Validator::Any(v) => f.debug_tuple("Validator::Any").field(v).finish(),
            Validator::Custom(_) => write!(f, "Validator::Custom(<opaque>)"),
        }
    }
}

impl Validator {
    pub fn starts_with(prefix: String) -> Self {
        Validator::StartsWith(prefix)
    }

    pub fn ends_with(suffix: String) -> Self {
        Validator::EndsWith(suffix)
    }

    pub fn contains(substring: String) -> Self {
        Validator::Contains(substring)
    }

    pub fn regex(regex: Regex) -> Self {
        Validator::Regex(regex)
    }

    pub fn negate(validator: Validator) -> Self {
        Validator::Not(Box::new(validator))
    }

    pub fn all(validators: Vec<Validator>) -> Self {
        Validator::All(Box::new(validators))
    }

    pub fn any(validators: Vec<Validator>) -> Self {
        Validator::Any(Box::new(validators))
    }

    pub fn custom(validator: Arc<dyn ValidatorTrait + Send + Sync>) -> Self {
        Validator::Custom(validator)
    }

    /// Adds a new validator to the current validator.
    /// If the current validator is `All` or `Any`, it appends to the existing list.
    /// If the current validator is a single validator, it wraps it in an `All` validator with the new one.
    pub fn add(&mut self, validator: Validator) {
        match self {
            Validator::All(validators) => validators.push(validator),
            Validator::Any(validators) => validators.push(validator),
            _ => {
                *self = Validator::All(Box::new(vec![self.clone(), validator]));
            }
        }
    }
}

impl PartialEq for Validator {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Validator::None, Validator::None) => true,
            (Validator::StartsWith(a), Validator::StartsWith(b)) => a == b,
            (Validator::EndsWith(a), Validator::EndsWith(b)) => a == b,
            (Validator::Contains(a), Validator::Contains(b)) => a == b,
            (Validator::Regex(a), Validator::Regex(b)) => a.as_str() == b.as_str(),
            (Validator::Not(a), Validator::Not(b)) => a == b,
            (Validator::All(a), Validator::All(b)) => a == b,
            (Validator::Any(a), Validator::Any(b)) => a == b,
            (Validator::Custom(a), Validator::Custom(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl ValidatorTrait for Validator {
    fn call(&self, data: &str) -> bool {
        match self {
            Validator::None => true,
            Validator::StartsWith(prefix) => data.starts_with(prefix),
            Validator::EndsWith(suffix) => data.ends_with(suffix),
            Validator::Contains(substring) => data.contains(substring),
            Validator::Regex(regex) => regex.is_match(data),
            Validator::Not(validator) => !validator.call(data),
            Validator::All(validators) => validators.iter().all(|v| v.call(data)),
            Validator::Any(validators) => validators.iter().any(|v| v.call(data)),
            Validator::Custom(validator) => validator.call(data),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RawValidator;

impl RawValidator {
    /// Creates a new instance of RawValidator
    pub fn new() -> Self {
        RawValidator
    }

    /// Validates a raw JSON message and returns a boolean indicating validity
    pub fn check(&self, message: &Value) -> bool {
        // For now, we'll consider any valid JSON as valid
        // In a more complex implementation, we might check for specific fields or structure
        !message.is_null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use serde_json::json;

    #[test]
    fn test_validator_none() {
        let v = Validator::None;
        assert!(v.call("anything"));
    }

    #[test]
    fn test_validator_starts_with() {
        let v = Validator::starts_with("prefix".into());
        assert!(v.call("prefix_data"));
        assert!(!v.call("data_prefix"));
    }

    #[test]
    fn test_validator_ends_with() {
        let v = Validator::ends_with("suffix".into());
        assert!(v.call("data_suffix"));
        assert!(!v.call("suffix_data"));
    }

    #[test]
    fn test_validator_contains() {
        let v = Validator::contains("middle".into());
        assert!(v.call("some_middle_data"));
        assert!(!v.call("other_data"));
    }

    #[test]
    fn test_validator_regex() {
        let re = Regex::new(r"^\d+$").unwrap();
        let v = Validator::regex(re);
        assert!(v.call("12345"));
        assert!(!v.call("abc123"));
    }

    #[test]
    fn test_validator_negate() {
        let v = Validator::negate(Validator::starts_with("not".into()));
        assert!(v.call("is_allowed"));
        assert!(!v.call("not_allowed"));
    }

    #[test]
    fn test_validator_all() {
        let v = Validator::all(vec![
            Validator::starts_with("a".into()),
            Validator::ends_with("z".into()),
        ]);
        assert!(v.call("applez"));
        assert!(!v.call("apple"));
        assert!(!v.call("banana z"));
    }

    #[test]
    fn test_validator_any() {
        let v = Validator::any(vec![
            Validator::starts_with("a".into()),
            Validator::starts_with("b".into()),
        ]);
        assert!(v.call("apple"));
        assert!(v.call("banana"));
        assert!(!v.call("cherry"));
    }

    #[test]
    fn test_validator_add() {
        let mut v = Validator::starts_with("a".into());
        v.add(Validator::ends_with("z".into()));
        assert!(v.call("applez"));
        assert!(!v.call("apple"));

        let mut v_any = Validator::any(vec![Validator::starts_with("a".into())]);
        v_any.add(Validator::starts_with("b".into()));
        assert!(v_any.call("banana"));
    }

    #[test]
    fn test_raw_validator() {
        let rv = RawValidator::new();
        assert!(rv.check(&json!({"key": "value"})));
        assert!(!rv.check(&json!(null)));
    }

    #[test]
    fn test_validator_debug() {
        let v = Validator::starts_with("test".into());
        assert!(format!("{:?}", v).contains("Validator::StartsWith(\"test\")"));

        let custom = Validator::Custom(Arc::new(RawValidator::new()));
        assert_eq!(format!("{:?}", custom), "Validator::Custom(<opaque>)");
    }

    impl ValidatorTrait for RawValidator {
        fn call(&self, _data: &str) -> bool {
            true
        }
    }

    #[test]
    fn test_validator_partial_eq() {
        assert_eq!(Validator::None, Validator::None);
        assert_eq!(
            Validator::starts_with("a".into()),
            Validator::starts_with("a".into())
        );
        assert_ne!(
            Validator::starts_with("a".into()),
            Validator::starts_with("b".into())
        );

        let re1 = Regex::new("a").unwrap();
        let re2 = Regex::new("a").unwrap();
        assert_eq!(Validator::regex(re1), Validator::regex(re2));
    }
}

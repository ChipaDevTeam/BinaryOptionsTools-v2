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

#[derive(Debug, Clone)]
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

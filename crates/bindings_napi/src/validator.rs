use binary_options_tools::validator::Validator as CrateValidator;
use napi_derive::napi;
use regex::Regex;

use crate::error::{napi_err, BinaryErrorNode};

/// Internal representation of a message validator.
///
/// Mirrors [`binary_options_tools::validator::Validator`] but keeps the JS
/// facing type opaque so that the variants can be composed from JavaScript.
#[derive(Clone, Default)]
pub enum RawValidator {
    #[default]
    None,
    Regex(Regex),
    StartsWith(String),
    EndsWith(String),
    Contains(String),
    All(Vec<RawValidator>),
    Any(Vec<RawValidator>),
    Not(Box<RawValidator>),
}

impl From<RawValidator> for CrateValidator {
    fn from(validator: RawValidator) -> Self {
        match validator {
            RawValidator::None => CrateValidator::None,
            RawValidator::Regex(regex) => CrateValidator::Regex(regex),
            RawValidator::StartsWith(prefix) => CrateValidator::StartsWith(prefix),
            RawValidator::EndsWith(suffix) => CrateValidator::EndsWith(suffix),
            RawValidator::Contains(substring) => CrateValidator::Contains(substring),
            RawValidator::All(validators) => {
                CrateValidator::All(Box::new(validators.into_iter().map(Into::into).collect()))
            }
            RawValidator::Any(validators) => {
                CrateValidator::Any(Box::new(validators.into_iter().map(Into::into).collect()))
            }
            RawValidator::Not(validator) => CrateValidator::Not(Box::new((*validator).into())),
        }
    }
}

/// Filters raw WebSocket messages.
///
/// Instances are built through the static constructors and can be nested with
/// `all`, `any` and `ne`. A `Validator` created with `new Validator()` matches
/// every message.
///
/// Note: unlike the Python bindings there is no `custom` constructor. A
/// JavaScript callback cannot be invoked synchronously from the WebSocket
/// thread, so custom predicates have to be applied on the values yielded by
/// `RawHandler.subscribe()` instead.
#[napi]
#[derive(Clone, Default)]
pub struct Validator {
    pub(crate) inner: RawValidator,
}

impl From<Validator> for CrateValidator {
    fn from(validator: Validator) -> Self {
        validator.inner.into()
    }
}

#[napi]
impl Validator {
    /// Creates a validator that accepts every message.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Matches messages against a regular expression.
    #[napi(factory)]
    pub fn regex(pattern: String) -> napi::Result<Self> {
        let regex = Regex::new(&pattern).map_err(|e| napi_err(BinaryErrorNode::from(e)))?;
        Ok(Self {
            inner: RawValidator::Regex(regex),
        })
    }

    /// Matches messages containing `pattern`.
    #[napi(factory)]
    pub fn contains(pattern: String) -> Self {
        Self {
            inner: RawValidator::Contains(pattern),
        }
    }

    /// Matches messages starting with `pattern`.
    #[napi(factory)]
    pub fn starts_with(pattern: String) -> Self {
        Self {
            inner: RawValidator::StartsWith(pattern),
        }
    }

    /// Matches messages ending with `pattern`.
    #[napi(factory)]
    pub fn ends_with(pattern: String) -> Self {
        Self {
            inner: RawValidator::EndsWith(pattern),
        }
    }

    /// Negates `validator`.
    #[napi(factory)]
    pub fn ne(validator: &Validator) -> Self {
        Self {
            inner: RawValidator::Not(Box::new(validator.inner.clone())),
        }
    }

    /// Matches only when every validator in `validators` matches.
    #[napi(factory)]
    pub fn all(validators: Vec<&Validator>) -> Self {
        Self {
            inner: RawValidator::All(validators.into_iter().map(|v| v.inner.clone()).collect()),
        }
    }

    /// Matches when at least one validator in `validators` matches.
    #[napi(factory)]
    pub fn any(validators: Vec<&Validator>) -> Self {
        Self {
            inner: RawValidator::Any(validators.into_iter().map(|v| v.inner.clone()).collect()),
        }
    }

    /// Runs the validator against `message`.
    #[napi]
    pub fn check(&self, message: String) -> bool {
        use binary_options_tools::traits::ValidatorTrait;
        let validator: CrateValidator = self.inner.clone().into();
        validator.call(&message)
    }
}

use std::sync::Arc;

use binary_options_tools::validator::Validator as InnerValidator;
use regex::Regex;

use crate::error::UniError;

/// Validator for filtering WebSocket messages.
///
/// Provides various methods to validate messages using different strategies
/// like regex matching, prefix/suffix checking, and logical combinations.
#[derive(uniffi::Object, Clone)]
pub struct Validator {
    inner: InnerValidator,
}

#[uniffi::export]
impl Validator {
    /// Creates a default validator that accepts all messages.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::None,
        })
    }

    /// Creates a validator that uses regex pattern matching.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Regular expression pattern
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// # Match messages starting with a number
    /// validator = Validator.regex(r"^\d+")
    /// assert validator.check("123 message") == True
    /// assert validator.check("abc") == False
    /// ```
    #[uniffi::constructor]
    pub fn regex(pattern: String) -> Result<Arc<Self>, UniError> {
        let regex = Regex::new(&pattern)
            .map_err(|e| UniError::Validator(format!("Invalid regex pattern: {}", e)))?;
        Ok(Arc::new(Self {
            inner: InnerValidator::regex(regex),
        }))
    }

    /// Creates a validator that checks if messages start with a specific prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - String that messages should start with
    #[uniffi::constructor]
    pub fn starts_with(prefix: String) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::starts_with(prefix),
        })
    }

    /// Creates a validator that checks if messages end with a specific suffix.
    ///
    /// # Arguments
    ///
    /// * `suffix` - String that messages should end with
    #[uniffi::constructor]
    pub fn ends_with(suffix: String) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::ends_with(suffix),
        })
    }

    /// Creates a validator that checks if messages contain a specific substring.
    ///
    /// # Arguments
    ///
    /// * `substring` - String that should be present in messages
    #[uniffi::constructor]
    pub fn contains(substring: String) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::contains(substring),
        })
    }

    /// Creates a validator that negates another validator's result.
    ///
    /// # Arguments
    ///
    /// * `validator` - Validator whose result should be negated
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// # Match messages that don't contain "error"
    /// v = Validator.ne(Validator.contains("error"))
    /// assert v.check("success message") == True
    /// assert v.check("error occurred") == False
    /// ```
    #[uniffi::constructor]
    pub fn ne(validator: Arc<Validator>) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::negate(validator.inner.clone()),
        })
    }

    /// Creates a validator that requires all input validators to match.
    ///
    /// # Arguments
    ///
    /// * `validators` - List of validators that all must match
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// # Match messages that start with "Hello" and end with "World"
    /// v = Validator.all([
    ///     Validator.starts_with("Hello"),
    ///     Validator.ends_with("World")
    /// ])
    /// assert v.check("Hello Beautiful World") == True
    /// assert v.check("Hello Beautiful") == False
    /// ```
    #[uniffi::constructor]
    pub fn all(validators: Vec<Arc<Validator>>) -> Arc<Self> {
        let inner_validators = validators.iter().map(|v| v.inner.clone()).collect();
        Arc::new(Self {
            inner: InnerValidator::all(inner_validators),
        })
    }

    /// Creates a validator that requires at least one input validator to match.
    ///
    /// # Arguments
    ///
    /// * `validators` - List of validators where at least one must match
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// # Match messages containing either "success" or "completed"
    /// v = Validator.any([
    ///     Validator.contains("success"),
    ///     Validator.contains("completed")
    /// ])
    /// assert v.check("operation successful") == True
    /// assert v.check("task completed") == True
    /// assert v.check("in progress") == False
    /// ```
    #[uniffi::constructor]
    pub fn any(validators: Vec<Arc<Validator>>) -> Arc<Self> {
        let inner_validators = validators.iter().map(|v| v.inner.clone()).collect();
        Arc::new(Self {
            inner: InnerValidator::any(inner_validators),
        })
    }

    /// Checks if a message matches this validator's conditions.
    ///
    /// # Arguments
    ///
    /// * `message` - String to validate
    ///
    /// # Returns
    ///
    /// True if message matches the validator's conditions, False otherwise
    #[uniffi::method]
    pub fn check(&self, message: String) -> bool {
        use binary_options_tools::traits::ValidatorTrait;
        self.inner.call(&message)
    }
}

impl Validator {
    /// Returns the inner validator for internal use
    pub(crate) fn inner(&self) -> &InnerValidator {
        &self.inner
    }

    /// Creates a Validator from an inner validator which is used internally when converting `InnerValidator` to `Arc<Self>`, in such we need the dead code allowance.
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: InnerValidator) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self {
            inner: InnerValidator::None,
        }
    }
}

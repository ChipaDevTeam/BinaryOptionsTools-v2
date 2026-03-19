use std::sync::Arc;

use binary_options_tools::validator::Validator as InnerValidator;
use bo2_macros::uniffi_doc;
use regex::Regex;

use crate::error::UniError;

#[uniffi_doc(
    name = "Validator",
    path = "BinaryOptionsToolsUni/docs_json/validator.json"
)]
#[derive(uniffi::Object, Clone)]
pub struct Validator {
    inner: InnerValidator,
}

#[uniffi::export]
impl Validator {
    /// Creates a default validator that accepts all messages.
    #[uniffi_doc(
        name = "new",
        path = "BinaryOptionsToolsUni/docs_json/validator.json"
    )]
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::None,
        })
    }

    #[uniffi_doc(
        name = "regex",
        path = "BinaryOptionsToolsUni/docs_json/validator.json"
    )]
    #[uniffi::constructor]
    pub fn regex(pattern: String) -> Result<Arc<Self>, UniError> {
        let regex = Regex::new(&pattern)
            .map_err(|e| UniError::Validator(format!("Invalid regex pattern: {}", e)))?;
        Ok(Arc::new(Self {
            inner: InnerValidator::regex(regex),
        }))
    }

    #[uniffi_doc(
        name = "starts_with",
        path = "BinaryOptionsToolsUni/docs_json/validator.json"
    )]
    #[uniffi::constructor]
    pub fn starts_with(prefix: String) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::starts_with(prefix),
        })
    }

    #[uniffi_doc(
        name = "ends_with",
        path = "BinaryOptionsToolsUni/docs_json/validator.json"
    )]
    #[uniffi::constructor]
    pub fn ends_with(suffix: String) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::ends_with(suffix),
        })
    }

    #[uniffi_doc(
        name = "contains",
        path = "BinaryOptionsToolsUni/docs_json/validator.json"
    )]
    #[uniffi::constructor]
    pub fn contains(substring: String) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::contains(substring),
        })
    }

    #[uniffi_doc(name = "ne", path = "BinaryOptionsToolsUni/docs_json/validator.json")]
    #[uniffi::constructor]
    pub fn ne(validator: Arc<Validator>) -> Arc<Self> {
        Arc::new(Self {
            inner: InnerValidator::negate(validator.inner.clone()),
        })
    }

    #[uniffi_doc(name = "all", path = "BinaryOptionsToolsUni/docs_json/validator.json")]
    #[uniffi::constructor]
    pub fn all(validators: Vec<Arc<Validator>>) -> Arc<Self> {
        let inner_validators = validators.iter().map(|v| v.inner.clone()).collect();
        Arc::new(Self {
            inner: InnerValidator::all(inner_validators),
        })
    }

    #[uniffi_doc(name = "any", path = "BinaryOptionsToolsUni/docs_json/validator.json")]
    #[uniffi::constructor]
    pub fn any(validators: Vec<Arc<Validator>>) -> Arc<Self> {
        let inner_validators = validators.iter().map(|v| v.inner.clone()).collect();
        Arc::new(Self {
            inner: InnerValidator::any(inner_validators),
        })
    }

    #[uniffi_doc(
        name = "check",
        path = "BinaryOptionsToolsUni/docs_json/validator.json"
    )]
    #[uniffi::method]
    pub fn check(&self, message: String) -> bool {
        use binary_options_tools::traits::ValidatorTrait;
        self.inner.call(&message)
    }
}

impl Validator {
    pub(crate) fn inner(&self) -> &InnerValidator {
        &self.inner
    }

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

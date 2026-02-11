#![allow(dead_code)]

use std::sync::Arc;

use pyo3::{
    pyclass, pymethods,
    types::{PyAnyMethods, PyList},
    Bound, Py, PyAny, PyResult,
};
use regex::Regex;

use crate::error::BinaryResultPy;
use binary_options_tools::traits::ValidatorTrait;
use binary_options_tools::validator::Validator as CrateValidator;
use pyo3::Python;

#[pyclass]
#[derive(Clone)]
pub struct ArrayValidator(Vec<RawValidator>);

#[pyclass]
#[derive(Clone)]
pub struct BoxedValidator(Box<RawValidator>);

#[pyclass]
#[derive(Clone)]
pub struct RegexValidator {
    regex: Regex,
}

#[pyclass]
#[derive(Clone)]
pub struct PyCustom {
    custom: Arc<Py<PyAny>>,
}

#[pyclass]
#[derive(Clone)]
/// `RawValidator` provides a flexible way to filter WebSocket messages
/// within the Python API. It encapsulates various validation strategies,
/// including regular expressions, substring checks, and custom Python
/// callables.
///
/// This class is designed to be used with `RawHandler` to define which
/// incoming messages should be processed.
///
/// # Python Custom Validator Behavior
/// When using the `RawValidator.custom()` constructor:
/// - The provided Python callable (`func`) must accept exactly one string
///   argument, which will be the incoming WebSocket message data.
/// - The callable should return a boolean value (`True` or `False`).
/// - If the callable raises an exception, or if its return value cannot
///   be interpreted as a boolean, the validation will silently fail and
///   be treated as `False`. No Python exception will be propagated back
///   to the calling Python code at the point of validation.
pub enum RawValidator {
    None(),
    Regex(RegexValidator),
    StartsWith(String),
    EndsWith(String),
    Contains(String),
    All(ArrayValidator),
    Any(ArrayValidator),
    Not(BoxedValidator),
    Custom(PyCustom),
}

impl RawValidator {
    pub fn new_regex(regex: String) -> BinaryResultPy<Self> {
        let regex = Regex::new(&regex)?;
        Ok(Self::Regex(RegexValidator { regex }))
    }

    pub fn new_all(validators: Vec<RawValidator>) -> Self {
        Self::All(ArrayValidator(validators))
    }

    pub fn new_any(validators: Vec<RawValidator>) -> Self {
        Self::Any(ArrayValidator(validators))
    }

    pub fn new_not(validator: RawValidator) -> Self {
        Self::Not(BoxedValidator(Box::new(validator)))
    }

    pub fn new_contains(pattern: String) -> Self {
        Self::Contains(pattern)
    }

    pub fn new_starts_with(pattern: String) -> Self {
        Self::StartsWith(pattern)
    }

    pub fn new_ends_with(pattern: String) -> Self {
        Self::EndsWith(pattern)
    }
}

impl Default for RawValidator {
    fn default() -> Self {
        Self::None()
    }
}

impl ArrayValidator {
    // TODO: Restore validation methods when the new API supports it
    // fn validate_all(&self, message: &RawWebsocketMessage) -> bool {
    //     self.0.iter().all(|d| d.validate(message))
    // }

    // fn validate_any(&self, message: &RawWebsocketMessage) -> bool {
    //     self.0.iter().any(|d| d.validate(message))
    // }
}

// TODO: Restore BoxedValidator implementation when the new API supports it
// impl ValidatorTrait<RawWebsocketMessage> for BoxedValidator {
//     fn validate(&self, message: &RawWebsocketMessage) -> bool {
//         self.0.validate(message)
//     }
// }

// TODO: Restore RegexValidator implementation when the new API supports it
// impl ValidatorTrait<RawWebsocketMessage> for RegexValidator {
//     fn validate(&self, message: &RawWebsocketMessage) -> bool {
//         self.regex.is_match(&message.to_string())
//     }
// }

#[pymethods]
impl RawValidator {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    #[staticmethod]
    pub fn regex(pattern: String) -> PyResult<Self> {
        Ok(Self::new_regex(pattern)?)
    }

    #[staticmethod]
    pub fn contains(pattern: String) -> Self {
        Self::new_contains(pattern)
    }

    #[staticmethod]
    pub fn starts_with(pattern: String) -> Self {
        Self::new_starts_with(pattern)
    }

    #[staticmethod]
    pub fn ends_with(pattern: String) -> Self {
        Self::new_ends_with(pattern)
    }

    #[staticmethod]
    pub fn ne(validator: Bound<'_, RawValidator>) -> Self {
        let val = validator.get();
        Self::new_not(val.clone())
    }

    #[staticmethod]
    pub fn all(validator: Bound<'_, PyList>) -> PyResult<Self> {
        let val = validator.extract::<Vec<RawValidator>>()?;
        Ok(Self::new_all(val))
    }

    #[staticmethod]
    pub fn any(validator: Bound<'_, PyList>) -> PyResult<Self> {
        let val = validator.extract::<Vec<RawValidator>>()?;
        Ok(Self::new_any(val))
    }

    #[staticmethod]
    /// Creates a custom validator using a Python callable.
    ///
    /// The `func` callable will be invoked with the incoming WebSocket message
    /// as a single string argument. It must return `True` to validate the message
    /// or `False` otherwise.
    ///
    /// **Behavior on Error/Invalid Return:**
    /// If `func` raises an exception or returns a non-boolean value,
    /// the validation will silently fail and be treated as `False`.
    /// No exception will be propagated.
    ///
    /// # Arguments
    /// * `func` - A Python callable that accepts one string argument and returns a boolean.
    pub fn custom(func: Py<PyAny>) -> Self {
        Self::Custom(PyCustom {
            custom: Arc::new(func),
        })
    }

    pub fn check(&self, msg: String) -> bool {
        let validator: CrateValidator = self.clone().into();
        validator.call(&msg)
    }
}

impl RawValidator {
    fn call(&self, data: &str) -> bool {
        match self {
            RawValidator::None() => true,
            RawValidator::Regex(validator) => validator.regex.is_match(data),
            RawValidator::StartsWith(prefix) => data.starts_with(prefix),
            RawValidator::EndsWith(suffix) => data.ends_with(suffix),
            RawValidator::Contains(substring) => data.contains(substring),
            RawValidator::All(validators) => validators.0.iter().all(|v| v.call(data)),
            RawValidator::Any(validators) => validators.0.iter().any(|v| v.call(data)),
            RawValidator::Not(validator) => !validator.0.call(data),
            RawValidator::Custom(py_custom) => Python::attach(|py| {
                let func = py_custom.custom.as_ref();
                match func.call1(py, (data,)) {
                    Ok(result) => {
                        match result.extract::<bool>(py) {
                            Ok(b) => b,
                            Err(_) => false, // If we can't extract a bool, return false
                        }
                    }
                    Err(_) => false, // If the function call fails, return false
                }
            }),
        }
    }
}

impl From<RawValidator> for CrateValidator {
    fn from(validator: RawValidator) -> Self {
        match validator {
            RawValidator::None() => CrateValidator::None,
            RawValidator::Regex(regex_validator) => CrateValidator::Regex(regex_validator.regex),
            RawValidator::StartsWith(prefix) => CrateValidator::StartsWith(prefix),
            RawValidator::EndsWith(suffix) => CrateValidator::EndsWith(suffix),
            RawValidator::Contains(substring) => CrateValidator::Contains(substring),
            RawValidator::All(array_validator) => {
                let validators: Vec<CrateValidator> =
                    array_validator.0.into_iter().map(|v| v.into()).collect();
                CrateValidator::All(Box::new(validators))
            }
            RawValidator::Any(array_validator) => {
                let validators: Vec<CrateValidator> =
                    array_validator.0.into_iter().map(|v| v.into()).collect();
                CrateValidator::Any(Box::new(validators))
            }
            RawValidator::Not(boxed_validator) => {
                let validator: CrateValidator = (*boxed_validator.0).into();
                CrateValidator::Not(Box::new(validator))
            }
            RawValidator::Custom(py_custom) => {
                // Create a custom validator that calls the Python function
                let custom_validator = Arc::new(PyCustomValidator {
                    func: py_custom.custom.clone(),
                });
                CrateValidator::Custom(custom_validator)
            }
        }
    }
}

struct PyCustomValidator {
    func: Arc<Py<PyAny>>,
}

impl ValidatorTrait for PyCustomValidator {
    fn call(&self, data: &str) -> bool {
        Python::attach(|py| {
            let func = self.func.as_ref();
            match func.call1(py, (data,)) {
                Ok(result) => {
                    match result.extract::<bool>(py) {
                        Ok(b) => b,
                        Err(_) => false, // If we can't extract a bool, return false
                    }
                }
                Err(_) => false, // If the function call fails, return false
            }
        })
    }
}

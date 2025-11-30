#![allow(dead_code)]

use std::sync::Arc;

use pyo3::{
    Bound, Py, PyAny, PyResult, pyclass, pymethods,
    types::{PyAnyMethods, PyList},
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
            RawValidator::Custom(py_custom) => Python::with_gil(|py| {
                let func = py_custom.custom.as_ref();
                match func.call1(py, (data,)) {
                    Ok(result) => result.extract::<bool>(py).unwrap_or(false),
                    Err(_) => false,
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

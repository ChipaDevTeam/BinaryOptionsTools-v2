#![allow(dead_code)]

use std::sync::Arc;

use pyo3::{
    pyclass, pymethods,
    types::{PyAnyMethods, PyList},
    Bound, PyObject, PyResult,
};
use regex::Regex;

use crate::error::BinaryResultPy;

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
    custom: Arc<PyObject>,
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
    pub fn custom(func: PyObject) -> Self {
        Self::Custom(PyCustom {
            custom: Arc::new(func),
        })
    }

    pub fn check(&self, _msg: String) -> bool {
        // TODO: Restore validation logic when the new API supports it
        // For now, return true as a placeholder
        true
        // let raw = RawWebsocketMessage::from(msg);
        // self.validate(&raw)
    }
}

#![allow(non_snake_case)]

mod config;
mod error;
mod framework;
mod logs;
mod pocketoption;
mod runtime;
mod stream;
mod validator;

use config::PyConfig;
use error::{
    InvalidParameterError, NotAllowedError, PocketOptionError, TradeNotFoundError,
    UninitializedError,
};
use framework::{PyBot, PyContext, PyStrategy, PyVirtualMarket};
use logs::{start_tracing, LogBuilder, Logger, StreamLogsIterator, StreamLogsLayer};
use pocketoption::{RawHandle, RawHandler, RawPocketOption, RawStreamIterator, StreamIterator};
use pyo3::prelude::*;
use validator::RawValidator;

use crate::framework::Action;

#[pymodule(name = "BinaryOptionsToolsV2")]
fn BinaryOptionsTools(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConfig>()?;
    m.add_class::<StreamLogsIterator>()?;
    m.add_class::<StreamLogsLayer>()?;
    m.add_class::<RawPocketOption>()?;
    m.add_class::<Logger>()?;
    m.add_class::<LogBuilder>()?;
    m.add_class::<StreamIterator>()?;
    m.add_class::<RawStreamIterator>()?;
    m.add_class::<RawValidator>()?;
    m.add_class::<RawHandle>()?;
    m.add_class::<RawHandler>()?;
    m.add_class::<PyBot>()?;
    m.add_class::<PyStrategy>()?;
    m.add_class::<PyContext>()?;
    m.add_class::<Action>()?;
    m.add_class::<PyVirtualMarket>()?;

    m.add_function(wrap_pyfunction!(start_tracing, m)?)?;

    // Register custom exceptions
    m.add("PocketOptionError", m.py().get_type::<PocketOptionError>())?;
    m.add("TradeNotFoundError", m.py().get_type::<TradeNotFoundError>())?;
    m.add("UninitializedError", m.py().get_type::<UninitializedError>())?;
    m.add("NotAllowedError", m.py().get_type::<NotAllowedError>())?;
    m.add("InvalidParameterError", m.py().get_type::<InvalidParameterError>())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_enum_variants() {
        assert_eq!(format!("{:?}", Action::Call), "Call");
        assert_eq!(format!("{:?}", Action::Put), "Put");
    }

    #[test]
    fn test_py_config_defaults() {
        let config = PyConfig::default();
        assert_eq!(config.inner.max_allowed_loops, 0);
    }
}

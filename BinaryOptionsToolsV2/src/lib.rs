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
    Ok(())
}

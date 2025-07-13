#![allow(non_snake_case)]

mod config;
mod error;
mod logs;
mod pocketoption;
mod runtime;
mod stream;
mod validator;

// use config::PyConfig;
use logs::{start_tracing, LogBuilder, Logger, StreamLogsIterator, StreamLogsLayer};
use pocketoption::{RawPocketOption, RawStreamIterator, StreamIterator};
use pyo3::prelude::*;
use validator::RawValidator;

#[pymodule]
#[pyo3(name = "BinaryOptionsToolsV2")]
fn BinaryOptionsTools(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<StreamLogsIterator>()?;
    m.add_class::<StreamLogsLayer>()?;
    m.add_class::<RawPocketOption>()?;
    m.add_class::<Logger>()?;
    m.add_class::<LogBuilder>()?;
    m.add_class::<StreamIterator>()?;
    m.add_class::<RawStreamIterator>()?;
    m.add_class::<RawValidator>()?;
    // m.add_class::<PyConfig>()?;

    m.add_function(wrap_pyfunction!(start_tracing, m)?)?;
    Ok(())
}

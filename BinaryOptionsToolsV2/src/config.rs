use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use binary_options_tools::config::Config;
use std::time::Duration;
use url::Url;

#[pyclass]
#[derive(Clone, Default)]
pub struct PyConfig {
    pub inner: Config,
    pub(crate) url_cache: Vec<String>,
}

#[pymethods]
impl PyConfig {
    #[new]
    pub fn new() -> Self {
        let inner = Config::default();
        let url_cache = inner.urls.iter().map(|u| u.to_string()).collect();
        Self {
            inner,
            url_cache,
        }
    }

    #[getter]
    fn max_allowed_loops(&self) -> u32 {
        self.inner.max_allowed_loops
    }

    #[setter]
    fn set_max_allowed_loops(&mut self, value: u32) {
        self.inner.max_allowed_loops = value;
    }

    #[getter]
    fn sleep_interval(&self) -> u64 {
        self.inner.sleep_interval.as_millis() as u64
    }

    #[setter]
    fn set_sleep_interval(&mut self, value: u64) {
        self.inner.sleep_interval = Duration::from_millis(value);
    }

    #[getter]
    fn reconnect_time(&self) -> u64 {
        self.inner.reconnect_time.as_secs()
    }

    #[setter]
    fn set_reconnect_time(&mut self, value: u64) {
        self.inner.reconnect_time = Duration::from_secs(value);
    }

    #[getter]
    fn connection_initialization_timeout_secs(&self) -> u64 {
        self.inner.connection_initialization_timeout.as_secs()
    }

    #[setter]
    fn set_connection_initialization_timeout_secs(&mut self, value: u64) {
        self.inner.connection_initialization_timeout = Duration::from_secs(value);
    }

    #[getter]
    fn timeout_secs(&self) -> u64 {
        self.inner.timeout.as_secs()
    }

    #[setter]
    fn set_timeout_secs(&mut self, value: u64) {
        self.inner.timeout = Duration::from_secs(value);
    }

    #[getter]
    fn urls(&self) -> Vec<String> {
        self.url_cache.clone()
    }

    #[setter]
    fn set_urls(&mut self, value: Vec<String>) -> PyResult<()> {
        let mut parsed_urls = Vec::new();
        let mut errors = Vec::new();

        for url_str in value {
            match Url::parse(&url_str) {
                Ok(url) => parsed_urls.push(url),
                Err(_) => errors.push(url_str),
            }
        }

        if !errors.is_empty() {
            return Err(PyValueError::new_err(format!(
                "Invalid URLs provided: {}",
                errors.join(", ")
            )));
        }

        self.inner.urls = parsed_urls;
        self.url_cache = self.inner.urls.iter().map(|u| u.to_string()).collect();
        Ok(())
    }
}

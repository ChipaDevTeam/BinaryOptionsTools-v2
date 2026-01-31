use pyo3::prelude::*;
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
    fn set_urls(&mut self, value: Vec<String>) {
        self.inner.urls = value
            .into_iter()
            .filter_map(|u| Url::parse(&u).ok())
            .collect();
        self.url_cache = self.inner.urls.iter().map(|u| u.to_string()).collect();
    }
}

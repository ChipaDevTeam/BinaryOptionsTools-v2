// use pyo3::prelude::*;
// use std::collections::HashSet;
// use std::time::Duration;
// use url::Url;

// use crate::error::BinaryResultPy;

// #[pyclass]
// #[derive(Clone, Default)]
// pub struct PyConfig {
//     #[pyo3(get, set)]
//     pub max_allowed_loops: u32,
//     #[pyo3(get, set)]
//     pub sleep_interval: u64,
//     #[pyo3(get, set)]
//     pub reconnect_time: u64,
//     #[pyo3(get, set)]
//     pub connection_initialization_timeout_secs: u64,
//     #[pyo3(get, set)]
//     pub timeout_secs: u64,
//     #[pyo3(get, set)]
//     pub urls: Vec<String>,
// }

// #[pymethods]
// impl PyConfig {
//     #[new]
//     pub fn new() -> Self {
//         Self {
//             max_allowed_loops: 100,
//             sleep_interval: 100,
//             reconnect_time: 5,
//             connection_initialization_timeout_secs: 30,
//             timeout_secs: 30,
//             urls: Vec::new(),
//         }
//     }

// }

// impl PyConfig {
//     // TODO: Restore config builder when the new API supports it
//     // pub fn build(&self) -> BinaryResultPy<ConfigBuilder<PocketData, WebSocketMessage, ()>> {
//     //     let urls: Result<Vec<Url>, url::ParseError> = self
//     //         .urls
//     //         .iter()
//     //         .map(|url| Url::parse(url))
//     //         .collect();

//     //     let config = ConfigBuilder::new()
//     //     .max_allowed_loops(self.max_allowed_loops)
//     //     .sleep_interval(self.sleep_interval)
//     //     .reconnect_time(self.reconnect_time)
//     //     .timeout(Duration::from_secs(self.timeout_secs))
//     //     .default_connection_url(HashSet::from_iter(urls.map_err(|e| {
//     //         BinaryOptionsToolsError::from(e)
//     //     })?));
//     //     Ok(config)
//     // }
// }

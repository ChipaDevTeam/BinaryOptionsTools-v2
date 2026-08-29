use binary_options_tools::closeoption::CloseOption;
use pyo3::{pyclass, pymethods, Bound, PyAny, PyResult, Python, PyErr, IntoPyObjectExt};
use pyo3_async_runtimes::tokio::future_into_py;

use crate::error::BinaryErrorPy;

/// Raw CloseOption client for Python bindings
#[pyclass(name = "RawCloseOption")]
pub struct RawCloseOption {
    inner: CloseOption,
    runtime: tokio::runtime::Runtime,
}
#[pymethods]
impl RawCloseOption {
    #[new]
    #[pyo3(signature = (token, sid, public_code, hidden_code, demo, url, config))]
    fn new(
        token: String,
        sid: String,
        public_code: String,
        hidden_code: String,
        demo: bool,
        url: String,
        config: Option<crate::config::PyConfig>,
    ) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = rt.block_on(async {
            let mut builder = binary_options_tools::closeoption::State::builder()
                .token(token)
                .sid(sid)
                .public_code(public_code)
                .hidden_code(hidden_code)
                .demo(demo);
            if !url.is_empty() {
                builder = builder.ws_url(url);
            }
            if let Some(cfg) = config {
                if let Some(proxy) = cfg.inner.proxy {
                    builder = builder.proxy(proxy);
                }
                if let Some(user_agent) = cfg.inner.user_agent {
                    builder = builder.user_agent(user_agent);
                }
                if let Some(origin) = cfg.inner.origin {
                    builder = builder.origin(origin);
                }
            }
            let state = builder.build().map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()));
            let state = match state {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let client = CloseOption::from_state(state).await.map_err(BinaryErrorPy::from)?;
            Ok(client)
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self { inner: client, runtime: rt })
    }

    fn connect(&mut self) -> PyResult<()> {
        // Already connected in new()
        Ok(())
    }

    pub fn buy<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        amount: f64,
        time: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .buy(&asset, amount, time)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn sell<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        amount: f64,
        time: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .sell(&asset, amount, time)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn check_win<'py>(&self, py: Python<'py>, order_id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .check_win(&order_id)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn balance<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .balance()
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn candles<'py>(&self, py: Python<'py>, asset: String, period: u32) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .get_candles(&asset, period, 100)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn get_candles<'py>(&self, py: Python<'py>, asset: String, period: u32, count: u32) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .get_candles(&asset, period, count)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn send_raw<'py>(&self, py: Python<'py>, message: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .send_raw(&message)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn active_assets<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .active_assets()
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn get_server_time<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .get_server_time()
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn shutdown<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            client
                .shutdown()
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| ().into_py_any(py))
        })
    }

    pub fn payout<'py>(&self, py: Python<'py>, asset: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .payout(&asset)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn history<'py>(&self, py: Python<'py>, limit: u32) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .history(limit)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn opened_deals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .opened_deals()
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn closed_deals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        future_into_py(py, async move {
            let res = client
                .closed_deals()
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Python::attach(|py| deal.into_py_any(py))
        })
    }

    pub fn get_candles_live<'py>(&self, py: Python<'py>, asset: String, period: u32) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            Err::<String, _>(BinaryErrorPy::NotAllowed("get_candles_live not yet implemented".into())).map_err(|e| e.into())
        })
    }
    pub fn subscribe_raw<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            Err::<String, _>(BinaryErrorPy::NotAllowed("subscribe_raw not yet implemented".into())).map_err(|e| e.into())
        })
    }
    pub fn raw_handler<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            Err::<String, _>(BinaryErrorPy::NotAllowed("raw_handler not yet implemented".into())).map_err(|e| e.into())
        })
    }
}
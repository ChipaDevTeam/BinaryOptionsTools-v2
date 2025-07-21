use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use std::time::Duration;

use binary_options_tools::pocketoption_pre::candle::{Candle, SubscriptionType};
use binary_options_tools::pocketoption_pre::error::PocketResult;
use binary_options_tools::pocketoption_pre::pocket_client::PocketOption;
// use binary_options_tools::pocketoption::types::base::RawWebsocketMessage;
// use binary_options_tools::pocketoption::types::update::DataCandle;
// use binary_options_tools::pocketoption::ws::stream::StreamAsset;
// use binary_options_tools::reimports::FilteredRecieverStream;
use futures_util::stream::{BoxStream, Fuse};
use futures_util::StreamExt;
use pyo3::{pyclass, pymethods, Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use uuid::Uuid;

use crate::error::BinaryErrorPy;
use crate::runtime::get_runtime;
use crate::stream::next_stream;
use crate::validator::RawValidator;
use tokio::sync::Mutex;

#[pyclass]
#[derive(Clone)]
pub struct RawPocketOption {
    client: PocketOption,
}

#[pyclass]
pub struct StreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<Candle>>>>>,
}

#[pyclass]
pub struct RawStreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<String>>>>>,
}

#[pymethods]
impl RawPocketOption {
    #[new]
    #[pyo3(signature = (ssid))]
    pub fn new(ssid: String, py: Python<'_>) -> PyResult<Self> {
        let runtime = get_runtime(py)?;
        runtime.block_on(async move {
            let client = PocketOption::new(ssid).await.map_err(BinaryErrorPy::from)?;
            Ok(Self { client })
        })
    }

    #[staticmethod]
    #[pyo3(signature = (ssid, url))]
    pub fn new_with_url(py: Python<'_>, ssid: String, url: String) -> PyResult<Self> {
        let runtime = get_runtime(py)?;
        runtime.block_on(async move {
            let client = PocketOption::new_with_url(ssid, url)
                .await
                .map_err(BinaryErrorPy::from)?;
            Ok(Self { client })
        })
    }

    pub fn is_demo(&self) -> bool {
        self.client.is_demo()
    }

    pub fn buy<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        amount: f64,
        time: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .buy(asset, time, amount)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res.1).map_err(BinaryErrorPy::from)?;
            let result = vec![res.0.to_string(), deal];
            Python::with_gil(|py| result.into_py_any(py))
        })
    }

    pub fn sell<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        amount: f64,
        time: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .sell(asset, time, amount)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res.1).map_err(BinaryErrorPy::from)?;
            let result = vec![res.0.to_string(), deal];
            Python::with_gil(|py| result.into_py_any(py))
        })
    }

    pub fn check_win<'py>(&self, py: Python<'py>, trade_id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .result(Uuid::parse_str(&trade_id).map_err(BinaryErrorPy::from)?)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::with_gil(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub async fn get_deal_end_time(&self, _trade_id: String) -> PyResult<Option<i64>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "get_deal_end_time is work in progress and not yet available".into(),
        )
        .into())
    }

    pub fn get_candles<'py>(
        &self,
        _py: Python<'py>,
        _asset: String,
        _period: i64,
        _offset: i64,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API

        Err(BinaryErrorPy::NotAllowed(
            "get_candles is work in progress and not yet available".into(),
        )
        .into())
    }

    pub fn get_candles_advanced<'py>(
        &self,
        _py: Python<'py>,
        _asset: String,
        _period: i64,
        _offset: i64,
        _time: i64,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "get_candles_advanced is work in progress and not yet available".into(),
        )
        .into())
    }

    pub async fn balance(&self) -> f64 {
        self.client.balance().await
    }

    pub async fn closed_deals(&self) -> PyResult<String> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "closed_deals is work in progress and not yet available".into(),
        )
        .into())
    }

    pub async fn clear_closed_deals(&self) {
        // Work in progress - this feature is not yet implemented in the new API
        // No-op for now
    }

    pub async fn opened_deals(&self) -> PyResult<String> {
        let deals = self.client.get_opened_deals().await;
        Ok(serde_json::to_string(&deals).map_err(BinaryErrorPy::from)?)
    }

    pub async fn payout(&self) -> PyResult<String> {
        // Work in progress - this feature is not yet implemented in the new API
        match self.client.assets().await {
            Some(assets) => {
                let payouts: HashMap<&String, i32> = assets
                    .0
                    .iter()
                    .map(|(asset, symbol)| (asset, symbol.payout))
                    .collect();
                Ok(serde_json::to_string(&payouts).map_err(BinaryErrorPy::from)?)
            }
            None => Err(BinaryErrorPy::Uninitialized("Assets not initialized yet.".into()).into()),
        }
    }

    pub fn history<'py>(
        &self,
        _py: Python<'py>,
        _asset: String,
        _period: i64,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(
            BinaryErrorPy::NotAllowed("history is work in progress and not yet available".into())
                .into(),
        )
    }

    pub fn subscribe_symbol<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe(symbol, SubscriptionType::none())
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::with_gil(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    pub fn subscribe_symbol_chuncked<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        chunck_size: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe(symbol, SubscriptionType::chunk(chunck_size))
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::with_gil(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    pub fn subscribe_symbol_timed<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        time: Duration,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe(symbol, SubscriptionType::time(time))
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::with_gil(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    pub fn subscribe_symbol_time_aligned<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        time: Duration,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe(
                    symbol,
                    SubscriptionType::time_aligned(time).map_err(BinaryErrorPy::from)?,
                )
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::with_gil(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    pub fn send_raw_message<'py>(
        &self,
        _py: Python<'py>,
        _message: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "send_raw_message is work in progress and not yet available".into(),
        )
        .into())
    }

    pub fn create_raw_order<'py>(
        &self,
        _py: Python<'py>,
        _message: String,
        _validator: Bound<'py, RawValidator>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "create_raw_order is work in progress and not yet available".into(),
        )
        .into())
    }

    pub fn create_raw_order_with_timeout<'py>(
        &self,
        _py: Python<'py>,
        _message: String,
        _validator: Bound<'py, RawValidator>,
        _timeout: Duration,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "create_raw_order_with_timeout is work in progress and not yet available".into(),
        )
        .into())
    }

    pub fn create_raw_order_with_timeout_and_retry<'py>(
        &self,
        _py: Python<'py>,
        _message: String,
        _validator: Bound<'py, RawValidator>,
        _timeout: Duration,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "create_raw_order_with_timeout_and_retry is work in progress and not yet available"
                .into(),
        )
        .into())
    }

    #[pyo3(signature = (_message, _validator, _timeout=None))]
    pub fn create_raw_iterator<'py>(
        &self,
        _py: Python<'py>,
        _message: String,
        _validator: Bound<'py, RawValidator>,
        _timeout: Option<Duration>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Work in progress - this feature is not yet implemented in the new API
        Err(BinaryErrorPy::NotAllowed(
            "create_raw_iterator is work in progress and not yet available".into(),
        )
        .into())
    }

    pub fn get_server_time<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(
            py,
            async move { Ok(client.server_time().await.timestamp()) },
        )
    }
}

#[pymethods]
impl StreamIterator {
    fn __aiter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __anext__<'py>(&'py mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let stream = self.stream.clone();
        future_into_py(py, async move {
            let res = next_stream(stream, false).await;
            res.map(|res| serde_json::to_string(&res).unwrap_or_default())
        })
    }

    fn __next__<'py>(&'py self, py: Python<'py>) -> PyResult<String> {
        let runtime = get_runtime(py)?;
        let stream = self.stream.clone();
        runtime.block_on(async move {
            let res = next_stream(stream, true).await;
            res.map(|res| serde_json::to_string(&res).unwrap_or_default())
        })
    }
}

#[pymethods]
impl RawStreamIterator {
    fn __aiter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __anext__<'py>(&'py mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let stream = self.stream.clone();
        future_into_py(py, async move {
            let res = next_stream(stream, false).await;
            res.map(|res| res.to_string())
        })
    }

    fn __next__<'py>(&'py self, py: Python<'py>) -> PyResult<String> {
        let runtime = get_runtime(py)?;
        let stream = self.stream.clone();
        runtime.block_on(async move {
            let res = next_stream(stream, true).await;
            res.map(|res| res.to_string())
        })
    }
}

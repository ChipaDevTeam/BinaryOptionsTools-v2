use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use std::time::Duration;

use binary_options_tools::pocketoption::candle::{Candle, SubscriptionType};
use binary_options_tools::pocketoption::error::PocketResult;
use binary_options_tools::pocketoption::modules::subscriptions::{
    HistoryStreamEvent, HistoryStreamMode,
};
use binary_options_tools::pocketoption::pocket_client::PocketOption;
use binary_options_tools::utils::f64_to_decimal;
use binary_options_tools::validator::Validator as CrateValidator;
use binary_options_tools::validator::Validator;
use futures_util::stream::{BoxStream, Fuse};
use futures_util::StreamExt;
use pyo3::{pyclass, pymethods, Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::config::PyConfig;
use crate::error::BinaryErrorPy;
use crate::runtime::get_runtime;
use crate::stream::next_stream;
use crate::validator::RawValidator;
use tokio::sync::Mutex;

const CONNECTION_TIMEOUT_SECS: u64 = 20;

/// Convert a tungstenite message to a string
fn message_to_string(msg: &tungstenite::Message) -> String {
    match msg {
        tungstenite::Message::Text(text) => text.to_string(),
        tungstenite::Message::Binary(data) => String::from_utf8_lossy(data).into_owned(),
        _ => String::new(),
    }
}

/// Convert an Arc<Message> to a string
fn arc_message_to_string(msg: &std::sync::Arc<tungstenite::Message>) -> String {
    message_to_string(msg.as_ref())
}

/// Send a raw message and wait for the response
async fn send_raw_message_and_wait(
    client: &PocketOption,
    validator: RawValidator,
    message: String,
) -> PyResult<String> {
    // Convert RawValidator to CrateValidator
    let crate_validator: CrateValidator = validator.into();

    // Create a raw handler with the validator
    let handler = client
        .create_raw_handler(crate_validator, None)
        .await
        .map_err(BinaryErrorPy::from)?;

    // Send the message and wait for the next matching response
    let response = handler
        .send_and_wait(binary_options_tools::pocketoption::modules::raw::Outgoing::Text(message))
        .await
        .map_err(BinaryErrorPy::from)?;

    // Convert the response to a string
    Ok(arc_message_to_string(&response))
}

/// Parses the user-facing history stream mode string into the crate enum.
fn parse_history_stream_mode(mode: &str) -> PyResult<HistoryStreamMode> {
    match mode {
        "points" => Ok(HistoryStreamMode::Points),
        "ohlc" => Ok(HistoryStreamMode::Ohlc),
        other => Err(BinaryErrorPy::InvalidParameter(format!(
            "Invalid history stream mode '{other}'. Expected 'points' or 'ohlc'"
        ))
        .into()),
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct RawPocketOption {
    pub(crate) client: PocketOption,
}

#[pyclass]
pub struct StreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<Candle>>>>>,
}

/// A single raw price point emitted by `subscribe_points`.
#[derive(Clone, Debug, Serialize)]
pub struct StreamPoint {
    pub asset: String,
    pub time: f64,
    pub price: f64,
}

/// Async/sync iterator over raw `(asset, time, price)` points.
#[pyclass]
pub struct PointStreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<StreamPoint>>>>>,
}

/// Async/sync iterator over lazy chart stream events (points or candles).
#[pyclass]
pub struct HistoryStreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<HistoryStreamEvent>>>>>,
}

#[pyclass]
pub struct RawStreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<String>>>>>,
}

#[pyclass]
pub struct RawHandle {
    handle: binary_options_tools::pocketoption::modules::raw::RawHandle,
}

#[pyclass]
pub struct RawHandler {
    handler: Arc<Mutex<binary_options_tools::pocketoption::modules::raw::RawHandler>>,
}

#[pymethods]
impl RawPocketOption {
    #[new]
    #[pyo3(signature = (ssid))]
    pub fn new(ssid: String, py: Python<'_>) -> PyResult<Self> {
        let runtime = get_runtime(py)?;
        runtime.block_on(async move {
            let client = tokio::time::timeout(
                Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                PocketOption::new(ssid),
            )
            .await
            .map_err(|_| BinaryErrorPy::NotAllowed("Connection timeout".into()))?
            .map_err(BinaryErrorPy::from)?;
            Ok(Self { client })
        })
    }

    #[staticmethod]
    pub fn create<'py>(ssid: String, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            let client = tokio::time::timeout(
                Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                PocketOption::new(ssid),
            )
            .await
            .map_err(|_| BinaryErrorPy::NotAllowed("Connection timeout".into()))?
            .map_err(BinaryErrorPy::from)?;
            Ok(RawPocketOption { client })
        })
    }

    #[staticmethod]
    #[pyo3(signature = (ssid, url))]
    pub fn new_with_url(py: Python<'_>, ssid: String, url: String) -> PyResult<Self> {
        let runtime = get_runtime(py)?;
        runtime.block_on(async move {
            let client = tokio::time::timeout(
                Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                PocketOption::new_with_url(ssid, url),
            )
            .await
            .map_err(|_| BinaryErrorPy::NotAllowed("Connection timeout".into()))?
            .map_err(BinaryErrorPy::from)?;
            Ok(Self { client })
        })
    }

    #[staticmethod]
    pub fn create_with_url<'py>(
        ssid: String,
        url: String,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            let client = tokio::time::timeout(
                Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                PocketOption::new_with_url(ssid, url),
            )
            .await
            .map_err(|_| BinaryErrorPy::NotAllowed("Connection timeout".into()))?
            .map_err(BinaryErrorPy::from)?;
            Ok(RawPocketOption { client })
        })
    }

    #[staticmethod]
    #[pyo3(signature = (ssid, config))]
    pub fn new_with_config(py: Python<'_>, ssid: String, config: PyConfig) -> PyResult<Self> {
        let runtime = get_runtime(py)?;
        runtime.block_on(async move {
            PocketOption::new_with_config(ssid, config.inner)
                .await
                .map(|client| Self { client })
                .map_err(|e| BinaryErrorPy::from(e).into())
        })
    }

    #[staticmethod]
    pub fn create_with_config<'py>(
        ssid: String,
        config: PyConfig,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            PocketOption::new_with_config(ssid, config.inner)
                .await
                .map(|client| RawPocketOption { client })
                .map_err(|e| BinaryErrorPy::from(e).into())
        })
    }

    pub fn wait_for_assets<'py>(
        &self,
        py: Python<'py>,
        timeout_secs: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let duration = Duration::from_secs_f64(timeout_secs);
        future_into_py(py, async move {
            client
                .wait_for_assets(duration)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    pub fn is_demo(&self) -> bool {
        self.client.is_demo()
    }

    /// Returns true if the client is currently connected to the WebSocket server.
    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn buy<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        amount: f64,
        time: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let decimal_amount = f64_to_decimal(amount)
            .ok_or_else(|| BinaryErrorPy::NotAllowed(format!("Invalid amount: {}", amount)))?;
        future_into_py(py, async move {
            let res = client
                .buy(asset, time, decimal_amount)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res.1).map_err(BinaryErrorPy::from)?;
            let result = vec![res.0.to_string(), deal];
            Python::attach(|py| result.into_py_any(py))
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
        let decimal_amount = f64_to_decimal(amount)
            .ok_or_else(|| BinaryErrorPy::NotAllowed(format!("Invalid amount: {}", amount)))?;
        future_into_py(py, async move {
            let res = client
                .sell(asset, time, decimal_amount)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res.1).map_err(BinaryErrorPy::from)?;
            let result = vec![res.0.to_string(), deal];
            Python::attach(|py| result.into_py_any(py))
        })
    }

    pub fn check_win<'py>(&self, py: Python<'py>, trade_id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .result(Uuid::parse_str(&trade_id).map_err(BinaryErrorPy::from)?)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn get_deal_end_time<'py>(
        &self,
        py: Python<'py>,
        trade_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let uuid = Uuid::parse_str(&trade_id).map_err(BinaryErrorPy::from)?;

            let deal = match client.get_closed_deal(uuid).await {
                Some(deal) => Some(deal),
                None => client.get_opened_deal(uuid).await,
            };

            Ok(deal.map(|d| d.close_timestamp.timestamp()))
        })
    }

    /// Gets historical candle data for a specific asset and period.
    pub fn candles<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        period: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .candles(asset, period)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn get_candles<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        period: i64,
        offset: i64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .get_candles(asset, period, offset)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn get_candles_advanced<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        period: i64,
        offset: i64,
        time: i64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .get_candles_advanced(asset, period, time, offset)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn balance<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let balance = client.balance().await;
            Ok(balance.to_f64().unwrap_or_default())
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn open_pending_order<'py>(
        &self,
        py: Python<'py>,
        open_type: u32,
        amount: f64,
        asset: String,
        open_time: String,
        open_price: f64,
        timeframe: u32,
        min_payout: u32,
        command: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let decimal_amount = f64_to_decimal(amount)
            .ok_or_else(|| BinaryErrorPy::NotAllowed(format!("Invalid amount: {}", amount)))?;
        let decimal_open_price = f64_to_decimal(open_price).ok_or_else(|| {
            BinaryErrorPy::NotAllowed(format!("Invalid open price: {}", open_price))
        })?;
        future_into_py(py, async move {
            let res = client
                .open_pending_order(
                    open_type,
                    decimal_amount,
                    asset,
                    open_time,
                    decimal_open_price,
                    timeframe,
                    min_payout,
                    command,
                )
                .await
                .map_err(BinaryErrorPy::from)?;
            let order = serde_json::to_string(&res).map_err(BinaryErrorPy::from)?;
            Ok(order)
        })
    }

    pub fn cancel_pending_order<'py>(
        &self,
        py: Python<'py>,
        ticket: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .cancel_pending_order(ticket)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                let result = serde_json::json!({
                    "ticket": res,
                    "status": "cancelled"
                });
                serde_json::to_string(&result)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn cancel_pending_orders<'py>(
        &self,
        py: Python<'py>,
        tickets: Vec<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .cancel_pending_orders(tickets)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                let result = serde_json::json!({
                    "cancelled": res
                });
                serde_json::to_string(&result)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn closed_deals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let deals = client.get_closed_deals().await;
            Python::attach(|py| {
                serde_json::to_string(&deals)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn get_closed_deal<'py>(
        &self,
        py: Python<'py>,
        trade_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let uuid = Uuid::parse_str(&trade_id).map_err(BinaryErrorPy::from)?;
            if let Some(deal) = client.get_closed_deal(uuid).await {
                let res = serde_json::to_string(&deal).map_err(BinaryErrorPy::from)?;
                Ok(Some(res))
            } else {
                Ok(None)
            }
        })
    }

    pub fn clear_closed_deals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            client.clear_closed_deals().await;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    pub fn opened_deals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let deals = client.get_opened_deals().await;
            let res = serde_json::to_string(&deals).map_err(BinaryErrorPy::from)?;
            Ok(res)
        })
    }

    pub fn get_opened_deal<'py>(
        &self,
        py: Python<'py>,
        trade_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let uuid = Uuid::parse_str(&trade_id).map_err(BinaryErrorPy::from)?;
            if let Some(deal) = client.get_opened_deal(uuid).await {
                let res = serde_json::to_string(&deal).map_err(BinaryErrorPy::from)?;
                Ok(Some(res))
            } else {
                Ok(None)
            }
        })
    }

    pub fn payout<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            match client.assets().await {
                Some(assets) => {
                    let payouts: HashMap<&String, i32> = assets
                        .0
                        .iter()
                        .filter_map(|(asset, symbol)| {
                            if symbol.is_active {
                                Some((asset, symbol.payout))
                            } else {
                                None
                            }
                        })
                        .collect();
                    let res = serde_json::to_string(&payouts).map_err(BinaryErrorPy::from)?;
                    Ok(res)
                }
                None => {
                    Err(BinaryErrorPy::Uninitialized("Assets not initialized yet.".into()).into())
                }
            }
        })
    }

    pub fn active_assets<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            match client.active_assets().await {
                Some(assets) => {
                    let res = serde_json::to_string(&assets).map_err(BinaryErrorPy::from)?;
                    Ok(res)
                }
                None => {
                    Err(BinaryErrorPy::Uninitialized("Assets not initialized yet.".into()).into())
                }
            }
        })
    }

    pub fn history<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        period: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .candles(asset, period)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    /// Gets Pocket-style merged history points for a specific asset and period.
    ///
    /// Returns the same point stream Pocket Option builds for its chart edge
    /// (raw ticks plus synthetic points expanded from server candles),
    /// serialized as a JSON list of `{asset, time, price}` objects.
    pub fn history_points<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        period: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .history_points(asset, period)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    /// Gets closed OHLC candles from Pocket Option's merged history flow.
    ///
    /// The newest (still developing) candle is excluded. Returns a JSON list
    /// of candle objects.
    pub fn history_ohlc<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        period: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .history_ohlc(asset, period)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    /// Compiles custom candlesticks from raw tick history.
    ///
    /// This method fetches raw tick data for the asset over the specified
    /// `lookback_period` and then aggregates those ticks into custom-sized
    /// candlesticks of `custom_period` seconds.
    ///
    /// Args:
    ///     asset (str): Trading symbol
    ///     custom_period (int): Desired candle duration in seconds
    ///     lookback_period (int): Number of seconds of tick history to fetch
    ///
    /// Returns:
    ///     List[Dict]: List of candle dictionaries with OHLC data
    pub fn compile_candles<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        custom_period: u32,
        lookback_period: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        if custom_period == 0 {
            return Err(BinaryErrorPy::InvalidParameter(
                "custom_period must be non-zero".to_string(),
            )
            .into());
        }
        if lookback_period == 0 {
            return Err(BinaryErrorPy::InvalidParameter(
                "lookback_period must be non-zero".to_string(),
            )
            .into());
        }
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .compile_candles(asset, custom_period, lookback_period)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
    }

    pub fn send_raw<'py>(&self, py: Python<'py>, message: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            client
                .send_raw(message)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    pub fn subscribe_raw<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let raw_stream = client.subscribe_raw().await.map_err(BinaryErrorPy::from)?;

            let boxed_stream = async_stream::stream! {
                tokio::pin!(raw_stream);
                while let Some(msg) = raw_stream.next().await {
                    yield Ok(arc_message_to_string(&msg));
                }
            }
            .boxed()
            .fuse();

            let stream = Arc::new(Mutex::new(boxed_stream));
            Python::attach(|py| RawStreamIterator { stream }.into_py_any(py))
        })
    }

    /// Subscribes to an asset's stream of raw price updates.
    ///
    /// Args:
    ///     symbol (str): Asset symbol to subscribe to.
    ///     subfor (bool): Whether to send the `subfor` frame after
    ///         `changeSymbol` (default True, the standard subscription).
    #[pyo3(signature = (symbol, subfor = true))]
    pub fn subscribe_symbol<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        subfor: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe_sub(symbol, SubscriptionType::none(), subfor)
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    /// Subscribes to an asset, aggregating every `chunk_size` updates into one candle.
    ///
    /// Args:
    ///     symbol (str): Asset symbol to subscribe to.
    ///     chunk_size (int): Number of updates aggregated per candle.
    ///     subfor (bool): Whether to send the `subfor` frame (default True).
    #[pyo3(signature = (symbol, chunk_size, subfor = true))]
    pub fn subscribe_symbol_chunked<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        chunk_size: usize,
        subfor: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe_sub(symbol, SubscriptionType::chunk(chunk_size), subfor)
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    /// Subscribes to an asset, aggregating updates into candles of `time` duration.
    ///
    /// Args:
    ///     symbol (str): Asset symbol to subscribe to.
    ///     time (timedelta): Candle duration.
    ///     subfor (bool): Whether to send the `subfor` frame (default True).
    #[pyo3(signature = (symbol, time, subfor = true))]
    pub fn subscribe_symbol_timed<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        time: Duration,
        subfor: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe_sub(symbol, SubscriptionType::time(time), subfor)
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    /// Subscribes to an asset with candles aligned to UTC time boundaries.
    ///
    /// Args:
    ///     symbol (str): Asset symbol to subscribe to.
    ///     time (timedelta): Candle duration (must divide 24h evenly).
    ///     subfor (bool): Whether to send the `subfor` frame (default True).
    #[pyo3(signature = (symbol, time, subfor = true))]
    pub fn subscribe_symbol_time_aligned<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        time: Duration,
        subfor: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let subscription = client
                .subscribe_sub(
                    symbol,
                    SubscriptionType::time_aligned(time).map_err(BinaryErrorPy::from)?,
                    subfor,
                )
                .await
                .map_err(BinaryErrorPy::from)?;

            let boxed_stream = subscription.to_stream().boxed().fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
        })
    }

    /// Subscribes to raw `(asset, time, price)` points using a raw handler.
    ///
    /// Uses `subscribeSymbol` with a keep-alive so the point feed survives
    /// reconnections. Returns a `PointStreamIterator`.
    pub fn subscribe_points<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let message = format!("42[\"subscribeSymbol\",\"{symbol}\"]");
            let keep_alive =
                binary_options_tools::pocketoption::modules::raw::Outgoing::Text(message.clone());
            let handler = client
                .create_raw_handler(Validator::Contains(symbol.clone()), Some(keep_alive))
                .await
                .map_err(BinaryErrorPy::from)?;

            handler
                .send_text(message)
                .await
                .map_err(BinaryErrorPy::from)?;

            let receiver = handler.subscribe();
            let asset = symbol.clone();
            let boxed_stream = async_stream::stream! {
                // Keep the handler alive for as long as the stream exists so
                // its keep-alive message continues to be sent on reconnects.
                let _handler = handler;

                while let Ok(msg) = receiver.recv().await {
                    let msg_str = message_to_string(msg.as_ref());

                    for point in parse_stream_points(&asset, &msg_str) {
                        yield Ok(point);
                    }
                }
            }
            .boxed()
            .fuse();
            let stream = Arc::new(Mutex::new(boxed_stream));

            Python::attach(|py| PointStreamIterator { stream }.into_py_any(py))
        })
    }

    /// Opens an early/lazy chart stream with one `changeSymbol` request.
    ///
    /// The stream first yields the history bootstrap and then matching live
    /// rows for the same asset. Returns a `HistoryStreamIterator`.
    ///
    /// Args:
    ///     symbol (str): Asset symbol.
    ///     period (int): Candle period in seconds.
    ///     mode (str): "points" for raw points or "ohlc" for closed candles.
    #[pyo3(signature = (symbol, period, mode = "points".to_string()))]
    pub fn subscribe_with_history_mode<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        period: u32,
        mode: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let mode = parse_history_stream_mode(&mode)?;
        future_into_py(py, async move {
            let stream = client
                .subscribe_with_history_mode(symbol, period, mode)
                .await
                .map_err(BinaryErrorPy::from)?
                .boxed()
                .fuse();
            let stream = Arc::new(Mutex::new(stream));

            Python::attach(|py| HistoryStreamIterator { stream }.into_py_any(py))
        })
    }

    pub fn send_raw_message<'py>(
        &self,
        py: Python<'py>,
        message: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            // Create a raw handler with a simple validator that matches everything
            let handler = client
                .create_raw_handler(Validator::None, None)
                .await
                .map_err(BinaryErrorPy::from)?;
            // Send the raw message without waiting for a response
            handler
                .send_text(message)
                .await
                .map_err(BinaryErrorPy::from)?;
            Ok(())
        })
    }

    pub fn create_raw_order<'py>(
        &self,
        py: Python<'py>,
        message: String,
        validator: Bound<'py, RawValidator>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let validator = validator.get().clone();
        future_into_py(py, async move {
            let response = send_raw_message_and_wait(&client, validator, message).await?;
            Python::attach(|py| response.into_py_any(py))
        })
    }

    pub fn create_raw_order_with_timeout<'py>(
        &self,
        py: Python<'py>,
        message: String,
        validator: Bound<'py, RawValidator>,
        timeout: Duration,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let validator = validator.get().clone();
        future_into_py(py, async move {
            let send_future = send_raw_message_and_wait(&client, validator, message);
            let response = tokio::time::timeout(timeout, send_future)
                .await
                .map_err(|_| {
                    Into::<pyo3::PyErr>::into(BinaryErrorPy::NotAllowed(
                        "Operation timed out".into(),
                    ))
                })?;
            Python::attach(|py| response?.into_py_any(py))
        })
    }

    pub fn create_raw_order_with_timeout_and_retry<'py>(
        &self,
        py: Python<'py>,
        message: String,
        validator: Bound<'py, RawValidator>,
        timeout: Duration,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let validator = validator.get().clone();
        future_into_py(py, async move {
            let max_retries = 3;
            let mut delay = Duration::from_millis(100);

            for retries in 0..max_retries {
                let send_future =
                    send_raw_message_and_wait(&client, validator.clone(), message.clone());
                match tokio::time::timeout(timeout, send_future).await {
                    Ok(Ok(response)) => {
                        return Python::attach(|py| response.into_py_any(py));
                    }
                    Ok(Err(e)) => {
                        if retries + 1 < max_retries {
                            tokio::time::sleep(delay).await;
                            delay = delay.saturating_mul(2);
                            continue;
                        }
                        return Err(e);
                    }
                    Err(_) => {
                        if retries + 1 < max_retries {
                            tokio::time::sleep(delay).await;
                            delay = delay.saturating_mul(2);
                            continue;
                        }
                        return Err(BinaryErrorPy::NotAllowed(
                            "Operation timed out after retries".into(),
                        )
                        .into());
                    }
                }
            }
            Err(BinaryErrorPy::NotAllowed("Operation failed after all retries".into()).into())
        })
    }

    pub fn create_raw_iterator<'py>(
        &self,
        py: Python<'py>,
        message: String,
        validator: Bound<'py, RawValidator>,
        timeout: Option<Duration>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let validator = validator.get().clone();
        future_into_py(py, async move {
            // Convert RawValidator to CrateValidator
            let crate_validator: CrateValidator = validator.into();

            // Create a raw handler with the validator
            let handler = client
                .create_raw_handler(crate_validator, None)
                .await
                .map_err(BinaryErrorPy::from)?;

            // Send the initial message
            handler
                .send_text(message)
                .await
                .map_err(BinaryErrorPy::from)?;

            // Create a stream from the handler's subscription
            let receiver = handler.subscribe();

            // Create a boxed stream that yields String values
            let boxed_stream = async_stream::stream! {
                // If a timeout is specified, apply it to the stream
                if let Some(timeout_duration) = timeout {
                    let start_time = std::time::Instant::now();
                    loop {
                        // Check if we've exceeded the timeout
                        if start_time.elapsed() >= timeout_duration {
                            break;
                        }

                        // Calculate remaining time for this iteration
                        let remaining_time = timeout_duration - start_time.elapsed();

                        // Try to receive a message with timeout
                        match tokio::time::timeout(remaining_time, receiver.recv()).await {
                            Ok(Ok(msg)) => {
                                // Convert the message to a string
                                let msg_str = msg.to_text().unwrap_or_default().to_string();
                                yield Ok(msg_str);
                            }
                            Ok(Err(_)) => break, // Channel closed
                            Err(_) => break, // Timeout
                        }
                    }
                } else {
                    // No timeout, just receive messages indefinitely
                    while let Ok(msg) = receiver.recv().await {
                        // Convert the message to a string
                        let msg_str = msg.to_text().unwrap_or_default().to_string();
                        yield Ok(msg_str);
                    }
                }
            }
            .boxed()
            .fuse();

            let stream = Arc::new(Mutex::new(boxed_stream));
            Python::attach(|py| RawStreamIterator { stream }.into_py_any(py))
        })
    }

    pub fn get_server_time<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(
            py,
            async move { Ok(client.server_time().await.timestamp()) },
        )
    }

    /// Commands the runner to shutdown.
    pub fn shutdown<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            client.shutdown().await.map_err(BinaryErrorPy::from)?;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    /// Disconnects the client while keeping the configuration intact.
    pub fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            client.disconnect().await.map_err(BinaryErrorPy::from)?;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    /// Establishes a connection after a manual disconnect.
    pub fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            client.connect().await.map_err(BinaryErrorPy::from)?;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    /// Disconnects and reconnects the client.
    pub fn reconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            client.reconnect().await.map_err(BinaryErrorPy::from)?;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    /// Unsubscribes from an asset's stream by asset name.
    pub fn unsubscribe<'py>(&self, py: Python<'py>, asset: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            client
                .unsubscribe(asset)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| py.None().into_py_any(py))
        })
    }

    /// Creates a raw handler with validator and optional keep-alive message.
    pub fn create_raw_handler<'py>(
        &self,
        py: Python<'py>,
        validator: Bound<'py, RawValidator>,
        keep_alive: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let validator = validator.get().clone();
        future_into_py(py, async move {
            let crate_validator: CrateValidator = validator.into();
            let keep_alive_msg =
                keep_alive.map(binary_options_tools::pocketoption::modules::raw::Outgoing::Text);
            let handler = client
                .create_raw_handler(crate_validator, keep_alive_msg)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                RawHandler {
                    handler: Arc::new(Mutex::new(handler)),
                }
                .into_py_any(py)
            })
        })
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
impl PointStreamIterator {
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
impl HistoryStreamIterator {
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
            res
        })
    }

    fn __next__<'py>(&'py self, py: Python<'py>) -> PyResult<String> {
        let runtime = get_runtime(py)?;
        let stream = self.stream.clone();
        runtime.block_on(async move {
            let res = next_stream(stream, true).await;
            res
        })
    }
}

/// Extracts every `(asset, time, price)` row for `asset` from an
/// `updateStream`-style JSON array payload. Non-matching rows and
/// non-array payloads are ignored.
fn parse_stream_points(asset: &str, payload: &str) -> Vec<StreamPoint> {
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(payload) else {
        return Vec::new();
    };

    rows.into_iter()
        .filter_map(|row| {
            let row = row.as_array()?;
            let row_asset = row.first()?.as_str()?;

            if row_asset != asset {
                return None;
            }

            Some(StreamPoint {
                asset: row_asset.to_string(),
                time: row.get(1)?.as_f64()?,
                price: row.get(2)?.as_f64()?,
            })
        })
        .collect()
}

#[pymethods]
impl RawHandle {
    /// Create a new RawHandler bound to the given validator
    pub fn create<'py>(
        &self,
        py: Python<'py>,
        validator: Bound<'py, RawValidator>,
        keep_alive_message: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = self.handle.clone();
        let validator = validator.get().clone();
        future_into_py(py, async move {
            let crate_validator: CrateValidator = validator.into();
            let keep_alive = keep_alive_message
                .map(binary_options_tools::pocketoption::modules::raw::Outgoing::Text);
            let handler = handle
                .create(crate_validator, keep_alive)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                RawHandler {
                    handler: Arc::new(Mutex::new(handler)),
                }
                .into_py_any(py)
            })
        })
    }

    /// Remove an existing handler by ID
    pub fn remove<'py>(&self, py: Python<'py>, id: String) -> PyResult<Bound<'py, PyAny>> {
        let handle = self.handle.clone();
        future_into_py(py, async move {
            let uuid = Uuid::parse_str(&id).map_err(BinaryErrorPy::from)?;
            let existed = handle.remove(uuid).await.map_err(BinaryErrorPy::from)?;
            Ok(existed)
        })
    }
}

#[pymethods]
impl RawHandler {
    /// Get the handler's ID
    pub fn id(&self) -> String {
        let handler = self.handler.blocking_lock();
        handler.id().to_string()
    }

    /// Send a text message
    pub fn send_text<'py>(&self, py: Python<'py>, text: String) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            let handler = handler.lock().await;
            handler.send_text(text).await.map_err(BinaryErrorPy::from)?;
            Ok(())
        })
    }

    /// Send a binary message
    pub fn send_binary<'py>(&self, py: Python<'py>, data: Vec<u8>) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            let handler = handler.lock().await;
            handler
                .send_binary(data)
                .await
                .map_err(BinaryErrorPy::from)?;
            Ok(())
        })
    }

    /// Send a message and wait for the next matching response
    pub fn send_and_wait<'py>(
        &self,
        py: Python<'py>,
        message: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            let handler = handler.lock().await;
            let msg = binary_options_tools::pocketoption::modules::raw::Outgoing::Text(message);
            let response = handler
                .send_and_wait(msg)
                .await
                .map_err(BinaryErrorPy::from)?;
            Ok(arc_message_to_string(&response))
        })
    }

    /// Wait for the next message that matches this handler's validator
    pub fn wait_next<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            let handler = handler.lock().await;
            let response = handler.wait_next().await.map_err(BinaryErrorPy::from)?;
            Ok(arc_message_to_string(&response))
        })
    }

    /// Subscribe to messages matching this handler's validator
    /// Returns an iterator that yields matching messages
    pub fn subscribe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.blocking_lock();
        let receiver = handler.subscribe();

        // Create a boxed stream that yields String values
        let boxed_stream = async_stream::stream! {
            while let Ok(msg) = receiver.recv().await {
                let msg_str = arc_message_to_string(&msg);
                yield Ok(msg_str);
            }
        }
        .boxed()
        .fuse();

        let stream = Arc::new(Mutex::new(boxed_stream));
        RawStreamIterator { stream }.into_bound_py_any(py)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_history_stream_mode, parse_stream_points, HistoryStreamMode};
    use pyo3::Python;

    #[test]
    fn parses_update_stream_points_for_requested_asset() {
        let points = parse_stream_points(
            "EURUSD_otc",
            r#"[["EURUSD_otc",1780724177.526,1.19234],["GBPUSD_otc",1780724177.700,1.31000]]"#,
        );

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].asset, "EURUSD_otc");
        assert_eq!(points[0].time, 1780724177.526);
        assert_eq!(points[0].price, 1.19234);
    }

    #[test]
    fn ignores_non_point_payloads() {
        assert!(parse_stream_points(
            "EURUSD_otc",
            r#"451-["updateStream",{"_placeholder":true,"num":0}]"#,
        )
        .is_empty());
        assert!(parse_stream_points("EURUSD_otc", r#"{"asset":"EURUSD_otc"}"#).is_empty());
    }

    #[test]
    fn subscribe_with_history_mode_accepts_points_and_ohlc_modes() {
        assert_eq!(
            parse_history_stream_mode("points").unwrap(),
            HistoryStreamMode::Points
        );
        assert_eq!(
            parse_history_stream_mode("ohlc").unwrap(),
            HistoryStreamMode::Ohlc
        );
    }

    #[test]
    fn subscribe_with_history_mode_rejects_unknown_mode() {
        Python::initialize();
        let error = parse_history_stream_mode("ticks").unwrap_err().to_string();
        assert!(error.contains("Expected 'points' or 'ohlc'"));
    }
}

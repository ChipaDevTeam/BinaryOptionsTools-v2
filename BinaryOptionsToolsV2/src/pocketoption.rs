use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use std::time::Duration;

use binary_options_tools::pocketoption::candle::{Candle, SubscriptionType};
use binary_options_tools::pocketoption::error::PocketResult;
use binary_options_tools::pocketoption::pocket_client::PocketOption;
use binary_options_tools::utils::f64_to_decimal;
use rust_decimal::prelude::ToPrimitive;
// use binary_options_tools::pocketoption::types::base::RawWebsocketMessage;
// use binary_options_tools::pocketoption::types::update::DataCandle;
// use binary_options_tools::pocketoption::ws::stream::StreamAsset;
// use binary_options_tools::reimports::FilteredRecieverStream;
use binary_options_tools::validator::Validator as CrateValidator;
use binary_options_tools::validator::Validator;
use futures_util::stream::{BoxStream, Fuse};
use futures_util::StreamExt;
use pyo3::{pyclass, pymethods, Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use uuid::Uuid;

use crate::config::PyConfig;
use crate::error::BinaryErrorPy;
use crate::runtime::get_runtime;
use crate::stream::next_stream;
use crate::validator::RawValidator;
use tokio::sync::Mutex;

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

#[pyclass]
#[derive(Clone)]
pub struct RawPocketOption {
    pub(crate) client: PocketOption,
}

#[pyclass]
pub struct StreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<Candle>>>>>,
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
            let client = tokio::time::timeout(Duration::from_secs(10), PocketOption::new(ssid))
                .await
                .map_err(|_| BinaryErrorPy::NotAllowed("Connection timeout".into()))?
                .map_err(BinaryErrorPy::from)?;
            Ok(Self { client })
        })
    }

    #[staticmethod]
    pub fn create<'py>(ssid: String, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            let client = tokio::time::timeout(Duration::from_secs(10), PocketOption::new(ssid))
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
                Duration::from_secs(10),
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
                Duration::from_secs(10),
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

            // Check if the deal is in closed deals first
            if let Some(deal) = client.get_closed_deal(uuid).await {
                return Ok(Some(deal.close_timestamp.timestamp()));
            }

            // If not found in closed deals, check opened deals
            if let Some(deal) = client.get_opened_deal(uuid).await {
                return Ok(Some(deal.close_timestamp.timestamp()));
            }

            // If not found in either, return None
            Ok(None) as PyResult<Option<i64>>
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
        // Work in progress - this feature is not yet implemented in the new API

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
        open_time: u32,
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

    pub fn payout<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            // Work in progress - this feature is not yet implemented in the new API
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
        // Work in progress - this feature is not yet implemented in the new API
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client
                .history(asset, period)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                serde_json::to_string(&res)
                    .map_err(BinaryErrorPy::from)?
                    .into_py_any(py)
            })
        })
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

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
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

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
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

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
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

            Python::attach(|py| StreamIterator { stream }.into_py_any(py))
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
            // Retry logic with exponential backoff
            let max_retries = 3;
            let mut delay = Duration::from_millis(100);

            for retries in 0..=max_retries {
                let send_future =
                    send_raw_message_and_wait(&client, validator.clone(), message.clone());
                match tokio::time::timeout(timeout, send_future).await {
                    Ok(Ok(response)) => {
                        return Python::attach(|py| response.into_py_any(py));
                    }
                    Ok(Err(e)) => {
                        if retries < max_retries {
                            tokio::time::sleep(delay).await;
                            delay *= 2; // Exponential backoff
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                    Err(_) => {
                        if retries < max_retries {
                            tokio::time::sleep(delay).await;
                            delay *= 2; // Exponential backoff
                            continue;
                        } else {
                            return Err(Into::<pyo3::PyErr>::into(BinaryErrorPy::NotAllowed(
                                "Operation timed out".into(),
                            )));
                        }
                    }
                }
            }
            unreachable!()
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

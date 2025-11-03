use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use std::time::Duration;

use binary_options_tools::pocketoption::candle::{Candle, SubscriptionType};
use binary_options_tools::pocketoption::error::PocketResult;
use binary_options_tools::pocketoption::pocket_client::PocketOption;
// use binary_options_tools::pocketoption::types::base::RawWebsocketMessage;
// use binary_options_tools::pocketoption::types::update::DataCandle;
// use binary_options_tools::pocketoption::ws::stream::StreamAsset;
// use binary_options_tools::reimports::FilteredRecieverStream;
use async_stream;
use binary_options_tools::validator::Validator as CrateValidator;
use binary_options_tools::validator::Validator;
use futures_util::StreamExt;
use futures_util::stream::{BoxStream, Fuse};
use pyo3::{Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python, pyclass, pymethods};
use pyo3_async_runtimes::tokio::future_into_py;
use tungstenite;
use uuid::Uuid;

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

#[pyclass]
pub struct RawHandlerRust {
    handler: Arc<Mutex<binary_options_tools::pocketoption::modules::raw::RawHandler>>,
}

#[pymethods]
impl RawHandlerRust {
    /// Send a text message through this handler
    pub fn send_text<'py>(&self, py: Python<'py>, message: String) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            handler
                .lock()
                .await
                .send_text(message)
                .await
                .map_err(BinaryErrorPy::from)?;
            Ok(())
        })
    }

    /// Send a binary message through this handler
    pub fn send_binary<'py>(&self, py: Python<'py>, data: Vec<u8>) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            handler
                .lock()
                .await
                .send_binary(data)
                .await
                .map_err(BinaryErrorPy::from)?;
            Ok(())
        })
    }

    /// Send a message and wait for the next matching response
    pub fn send_and_wait<'py>(&self, py: Python<'py>, message: String) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            let outgoing = binary_options_tools::pocketoption::modules::raw::Outgoing::Text(message);
            let response = handler
                .lock()
                .await
                .send_and_wait(outgoing)
                .await
                .map_err(BinaryErrorPy::from)?;
            let msg_str = response.to_text().unwrap_or_default().to_string();
            Python::attach(|py| msg_str.into_py_any(py))
        })
    }

    /// Wait for the next matching message
    pub fn wait_next<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            let response = handler
                .lock()
                .await
                .wait_next()
                .await
                .map_err(BinaryErrorPy::from)?;
            let msg_str = response.to_text().unwrap_or_default().to_string();
            Python::attach(|py| msg_str.into_py_any(py))
        })
    }

    /// Subscribe to messages matching this handler's validator
    pub fn subscribe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handler = self.handler.clone();
        future_into_py(py, async move {
            let receiver = {
                let handler_guard = handler.lock().await;
                handler_guard.subscribe()
            };

            // Create a boxed stream that yields String values
            let boxed_stream = async_stream::stream! {
                while let Ok(msg) = receiver.recv().await {
                    let msg_str = msg.to_text().unwrap_or_default().to_string();
                    yield Ok(msg_str);
                }
            }
            .boxed()
            .fuse();

            let stream = Arc::new(Mutex::new(boxed_stream));
            Python::attach(|py| RawStreamIterator { stream }.into_py_any(py))
        })
    }

    /// Get the handler's unique ID
    pub fn id(&self, py: Python<'_>) -> PyResult<String> {
        let runtime = get_runtime(py)?;
        let handler = self.handler.clone();
        runtime.block_on(async move {
            let handler_guard = handler.lock().await;
            Ok(handler_guard.id().to_string())
        })
    }
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
        future_into_py(py, async move {
            let res = client
                .sell(asset, time, amount)
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

    pub async fn balance(&self) -> f64 {
        self.client.balance().await
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
                    .filter_map(|(asset, symbol)| if symbol.is_active { Some((asset, symbol.payout)) } else { None })
                    .collect();
                Ok(serde_json::to_string(&payouts).map_err(BinaryErrorPy::from)?)
            }
            None => Err(BinaryErrorPy::Uninitialized("Assets not initialized yet.".into()).into()),
        }
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
            client.unsubscribe(asset).await.map_err(BinaryErrorPy::from)?;
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
            let keep_alive_msg = keep_alive.map(|msg| {
                binary_options_tools::pocketoption::modules::raw::Outgoing::Text(msg)
            });
            let handler = client
                .create_raw_handler(crate_validator, keep_alive_msg)
                .await
                .map_err(BinaryErrorPy::from)?;
            Python::attach(|py| {
                RawHandlerRust {
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
            res.map(|s| s)
        })
    }

    fn __next__<'py>(&'py self, py: Python<'py>) -> PyResult<String> {
        let runtime = get_runtime(py)?;
        let stream = self.stream.clone();
        runtime.block_on(async move {
            let res = next_stream(stream, true).await;
            res.map(|s| s)
        })
    }
}

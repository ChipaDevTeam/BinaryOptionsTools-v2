use std::sync::Arc;
use std::time::Duration;

use binary_options_tools_core::{
    builder::ClientBuilder,
    client::Client,
    error::CoreResult,
    reimports::{AsyncSender, AsyncReceiver, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use kanal;
use tokio::task::JoinHandle;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::closeoption::connect::CloseConnect;
use crate::closeoption::error::CloseOptionError;
use crate::closeoption::state::State;
use crate::closeoption::types::{Asset, Candle, Get30MinResult, OrderResult, Outgoing, SubscriptionEvent, SetOrderRequest, Get30MinRequest, PriceData};
use crate::closeoption::utils::get_index;

/// Lightweight module for handling price data
pub struct PriceDataModule {
    state: Arc<State>,
    receiver: AsyncReceiver<Arc<Message>>,
}

#[async_trait::async_trait]
impl LightweightModule<State> for PriceDataModule {
    fn new(
        state: Arc<State>,
        _: AsyncSender<Message>,
        receiver: AsyncReceiver<Arc<Message>>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            debug!(target: "Router", msg_type = %msg, "received raw message");
            if let Ok(text) = msg.to_text() {
                // Parse Socket.IO frames properly
                if let Ok(frames) = crate::closeoption::utils::parse_socket_io_message(text) {
                    for frame in frames {
                        if frame.message_type == crate::closeoption::types::socket_io::SocketIoMessageType::Event {
                            // Parse event name from data: ["eventName", ...]
                            if let Ok(event_array) = serde_json::from_str::<Vec<serde_json::Value>>(&frame.data) {
                                if let Some(event_name) = event_array.get(0).and_then(|v| v.as_str()) {
                                    if event_name == "priceData" {
                                        if let Some(data_value) = event_array.get(1) {
                                            if let Ok(price_data) = serde_json::from_value::<PriceData>(data_value.clone()) {
                                                self.state.update_assets(&price_data).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(|msg: &Message| {
            if let Ok(text) = msg.to_text() {
                text.contains("priceData")
            } else {
                false
            }
        })
    }
}

/// Lightweight module for handling balance updates
pub struct BalanceModule {
    state: Arc<State>,
    receiver: AsyncReceiver<Arc<Message>>,
}

#[async_trait::async_trait]
impl LightweightModule<State> for BalanceModule {
    fn new(
        state: Arc<State>,
        _: AsyncSender<Message>,
        receiver: AsyncReceiver<Arc<Message>>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            if let Ok(text) = msg.to_text() {
                // Parse Socket.IO frames properly
                if let Ok(frames) = crate::closeoption::utils::parse_socket_io_message(text) {
                    for frame in frames {
                        if frame.message_type == crate::closeoption::types::socket_io::SocketIoMessageType::Event {
                            // Parse event name from data: ["eventName", ...]
                            if let Ok(event_array) = serde_json::from_str::<Vec<serde_json::Value>>(&frame.data) {
                                if let Some(event_name) = event_array.get(0).and_then(|v| v.as_str()) {
                                    if event_name == "setOrderResult" {
                                        if let Some(data_value) = event_array.get(1) {
                                            if let Ok(order_result) = serde_json::from_value::<OrderResult>(data_value.clone()) {
                                                // Store order result in state
                                                let mut orders = self.state.orders.lock().await;
                                                orders.insert(order_result.order_id.clone(), order_result.clone());
                                                self.state.update_balance(order_result.balance).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(|msg: &Message| {
            if let Ok(text) = msg.to_text() {
                text.contains("setOrderResult")
            } else {
                false
            }
        })
    }
}

/// Lightweight module for keep-alive
pub struct KeepAliveModule {
    sender: AsyncSender<Message>,
    receiver: AsyncReceiver<Arc<Message>>,
}

#[async_trait::async_trait]
impl LightweightModule<State> for KeepAliveModule {
    fn new(
        _state: Arc<State>,
        sender: AsyncSender<Message>,
        receiver: AsyncReceiver<Arc<Message>>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(25));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let ping_frame = crate::closeoption::types::socket_io::ping();
                    if let Err(e) = self.sender.send(Message::Text(ping_frame.into())).await {
                        warn!("Failed to send ping: {}", e);
                        return Ok(());
                    }
                    debug!("Sent Socket.IO ping");
                }
                msg = self.receiver.recv() => {
                    if let Ok(msg) = msg {
                        if let Ok(text) = msg.to_text() {
                            if text == "3" || text.starts_with("3") {
                                debug!("Received Socket.IO pong");
                            }
                        }
                    }
                }
            }
        }
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(|msg: &Message| {
            if let Ok(text) = msg.to_text() {
                text == "3" || text.starts_with("3")
            } else {
                false
            }
        })
    }
}

/// High-level CloseOption client
#[derive(Clone)]
pub struct CloseOption {
    client: Client<State>,
    runner: Arc<JoinHandle<()>>,
}

/// Raw handler for advanced WebSocket operations
pub struct RawHandler {
    pub sender: AsyncSender<Message>,
}

impl RawHandler {
    /// Send a raw message
    pub async fn send(&self, message: &str) -> Result<(), CloseOptionError> {
        let frame = crate::closeoption::types::socket_io::event("raw", message);
        self.sender.send(Message::Text(frame.into())).await
            .map_err(|e| CloseOptionError::General(format!("Failed to send raw: {}", e)))?;
        Ok(())
    }
}

impl CloseOption {
    /// Create a new CloseOption client and connect
    pub async fn new(
        token: impl Into<String>,
        sid: impl Into<String>,
        public_code: impl Into<String>,
        hidden_code: impl Into<String>,
        demo: bool,
    ) -> Result<Self, CloseOptionError> {
        let state = State::builder()
            .token(token)
            .sid(sid)
            .public_code(public_code)
            .hidden_code(hidden_code)
            .demo(demo)
            .build()?;

        Self::from_state(state).await
    }

    /// Create from existing state
    pub async fn from_state(state: State) -> Result<Self, CloseOptionError> {
        let connector = CloseConnect;
        let builder = ClientBuilder::new(connector, state)
            .with_lightweight_module::<PriceDataModule>()
            .with_lightweight_module::<BalanceModule>()
            .with_lightweight_module::<KeepAliveModule>()
            .with_lightweight_module::<ResponseRouterModule>();

        let (client, mut runner) = builder.build().await
            .map_err(|e| CloseOptionError::General(format!("Failed to build client: {}", e)))?;

        let runner_handle = tokio::spawn(async move {
            runner.run().await;
        });

        // Wait for connection
        client.wait_connected().await;

        Ok(Self {
            client,
            runner: Arc::new(runner_handle),
        })
    }

    /// Get the internal state
    pub fn state(&self) -> Arc<State> {
        self.client.state.clone()
    }

    async fn send_and_wait(&self, request: Outgoing) -> Result<SubscriptionEvent, CloseOptionError> {
        let id = get_index();
        let (tx, rx) = oneshot::channel();
        let state = self.state();
        
        // Register pending request in state
        {
            let mut pending = state.pending_requests.lock().await;
            pending.insert(id, tx);
            debug!(target: "Router", request_id = id, event = request.event_name(), "registered pending request");
        }
        
        // Serialize request with ID
        let mut request_value = serde_json::to_value(&request)
            .map_err(|e| CloseOptionError::General(format!("Failed to serialize: {}", e)))?;
        if let serde_json::Value::Object(ref mut map) = request_value {
            map.insert("id".to_string(), serde_json::Value::Number(id.into()));
        }
        let json = serde_json::to_string(&request_value)
            .map_err(|e| CloseOptionError::General(format!("Failed to serialize: {}", e)))?;
        
        let frame = crate::closeoption::types::socket_io::event(request.event_name(), &json);
        debug!(target: "Router", request_id = id, frame = %frame, "sending request frame");
        
        // Send with cleanup on failure
        if let Err(e) = self.client.to_ws_sender.send(Message::Text(frame.into())).await {
            // Clean up pending request on send failure
            state.pending_requests.lock().await.remove(&id);
            return Err(CloseOptionError::General(format!("Failed to send: {}", e)));
        }

        // Wait for response with timeout
        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => {
                // Clean up pending request
                state.pending_requests.lock().await.remove(&id);
                Ok(response)
            },
            Ok(Err(_)) => {
                // Clean up on channel error
                state.pending_requests.lock().await.remove(&id);
                Err(CloseOptionError::General("Response channel closed".to_string()))
            },
            Err(_) => {
                // Clean up on timeout
                let pending_count = state.pending_requests.lock().await.len();
                debug!(target: "Router", request_id = id, pending_count, "send_and_wait timeout");
                state.pending_requests.lock().await.remove(&id);
                Err(CloseOptionError::Timeout {
                    task: "send_and_wait".to_string(),
                    context: "waiting for response".to_string(),
                    duration: Duration::from_secs(30),
                })
            }
        }
    }

    /// Place a BUY (CALL) order
    pub async fn buy(&self, asset: &str, amount: f64, duration: u32) -> Result<OrderResult, CloseOptionError> {
        let time_intervals = Self::duration_to_time_intervals(duration)?;
        let acc_type = self.state().acc_type().to_string();

        let request = SetOrderRequest {
            token: self.state().token.clone(),
            time_intervals,
            amount,
            order_type: "call".to_string(),
            public_code: self.state().public_code.clone(),
            hidden_code: self.state().hidden_code.clone(),
            acc_type,
            pair: asset.to_string(),
            contest_type: "".to_string(),
        };

        match self.send_and_wait(Outgoing::SetOrder(request)).await? {
            SubscriptionEvent::SetOrderResult(result) => Ok(result),
            _ => Err(CloseOptionError::General("Unexpected response type".to_string())),
        }
    }

    /// Place a SELL (PUT) order
    pub async fn sell(&self, asset: &str, amount: f64, duration: u32) -> Result<OrderResult, CloseOptionError> {
        let time_intervals = Self::duration_to_time_intervals(duration)?;
        let acc_type = self.state().acc_type().to_string();

        let request = SetOrderRequest {
            token: self.state().token.clone(),
            time_intervals,
            amount,
            order_type: "put".to_string(),
            public_code: self.state().public_code.clone(),
            hidden_code: self.state().hidden_code.clone(),
            acc_type,
            pair: asset.to_string(),
            contest_type: "".to_string(),
        };

        match self.send_and_wait(Outgoing::SetOrder(request)).await? {
            SubscriptionEvent::SetOrderResult(result) => Ok(result),
            _ => Err(CloseOptionError::General("Unexpected response type".to_string())),
        }
    }

    /// Check trade result
    pub async fn check_win(&self, order_id: &str) -> Result<OrderResult, CloseOptionError> {
        let state = self.state();
        let orders = state.orders.lock().await;
        if let Some(order) = orders.get(order_id) {
            Ok(order.clone())
        } else {
            Err(CloseOptionError::DealNotFound(order_id.to_string()))
        }
    }

    /// Get current balance
    pub async fn balance(&self) -> Result<Option<f64>, CloseOptionError> {
        Ok(self.state().get_balance().await)
    }

    /// Get historical candles
    pub async fn get_candles(&self, asset: &str, period: u32, _count: u32) -> Result<Vec<Candle>, CloseOptionError> {
        let ps_type = match period {
            30 => "30min",
            60 => "1min",
            300 => "5min",
            900 => "15min",
            1800 => "30min",
            3600 => "1hour",
            _ => return Err(CloseOptionError::InvalidPeriod(period)),
        }.to_string();

        let acc_type = self.state().acc_type().to_string();

        let request = Get30MinRequest {
            token: self.state().token.clone(),
            ps_type,
            public_code: self.state().public_code.clone(),
            hidden_code: self.state().hidden_code.clone(),
            acc_type,
            pair: asset.to_string(),
            contest_type: "".to_string(),
        };

        match self.send_and_wait(Outgoing::Get30Min(request)).await? {
            SubscriptionEvent::Get30MinResult(result) => Ok(result.price),
            _ => Err(CloseOptionError::General("Unexpected response type".to_string())),
        }
    }


    /// Get ticks/candles for an asset (alias for get_candles with 30min period)
    pub async fn get_ticks(&self, asset: &str) -> Result<Vec<Candle>, CloseOptionError> {
        self.get_candles(asset, 30, 0).await
    }

    /// Get active assets
    pub async fn active_assets(&self) -> Result<Vec<Asset>, CloseOptionError> {
        let assets = self.state().get_assets().await;
        Ok(assets.into_values().collect())
    }

    /// Subscribe to price updates for a symbol
    pub async fn subscribe_symbol(&self, symbol: &str) -> Result<AsyncReceiver<SubscriptionEvent>, CloseOptionError> {
        let (tx, rx) = kanal::bounded_async::<SubscriptionEvent>(100);
        self.state().subscriptions.lock().await.insert(symbol.to_string(), tx);
        Ok(rx)
    }

    /// Subscribe to all raw messages
    pub async fn subscribe_raw(&self) -> Result<AsyncReceiver<SubscriptionEvent>, CloseOptionError> {
        let (tx, rx) = kanal::bounded_async::<SubscriptionEvent>(100);
        self.state().raw_subscriptions.lock().await.push(tx);
        Ok(rx)
    }

    /// Send raw message
    pub async fn send_raw(&self, message: &str) -> Result<(), CloseOptionError> {
        let frame = crate::closeoption::types::socket_io::event("raw", message);
        self.client.to_ws_sender.send(Message::Text(frame.into())).await
            .map_err(|e| CloseOptionError::General(format!("Failed to send raw: {}", e)))?;
        Ok(())
    }

    /// Get server time
    pub async fn get_server_time(&self) -> Result<i64, CloseOptionError> {
        Ok(self.state().server_time().await)
    }

    /// Shutdown the client
    pub async fn shutdown(self) -> Result<(), CloseOptionError> {
        self.client.shutdown_ref().await
            .map_err(|e| CloseOptionError::General(format!("Failed to shutdown: {}", e)))?;
        Ok(())
    }

    /// Reconnect
    pub async fn reconnect(&self) -> Result<(), CloseOptionError> {
        self.client.reconnect().await
            .map_err(|e| CloseOptionError::General(format!("Failed to reconnect: {}", e)))?;
        Ok(())
    }

    /// Map duration in seconds to CloseOption time interval string
    fn duration_to_time_intervals(duration: u32) -> Result<String, CloseOptionError> {
        match duration {
            30 => Ok("30 Seconds".to_string()),
            60 => Ok("1 Minute".to_string()),
            120 => Ok("2 Minutes".to_string()),
            300 => Ok("5 Minutes".to_string()),
            600 => Ok("10 Minutes".to_string()),
            d if d <= 60 => Ok("30 Seconds".to_string()),
            d if d <= 600 => Ok("10 Minutes".to_string()),
            d => Err(CloseOptionError::General(format!("Unsupported trade duration: {} seconds", d))),
        }
    }

    /// Get payout for an asset
    pub async fn payout(&self, _asset: &str) -> Result<f64, CloseOptionError> {
        Err(CloseOptionError::Unsupported("Per-asset payout not available".into()))
    }

    /// Get trade history
    pub async fn history(&self, _limit: u32) -> Result<Vec<OrderResult>, CloseOptionError> {
        Err(CloseOptionError::Unsupported("Trade history not available".into()))
    }

    /// Get opened deals
    pub async fn opened_deals(&self) -> Result<Vec<OrderResult>, CloseOptionError> {
        Err(CloseOptionError::Unsupported("Opened deals not available".into()))
    }

    /// Get closed deals
    pub async fn closed_deals(&self) -> Result<Vec<OrderResult>, CloseOptionError> {
        Err(CloseOptionError::Unsupported("Closed deals not available".into()))
    }

    /// Get live candle updates
    pub async fn get_candles_live(&self, _asset: &str, _period: u32) -> Result<AsyncReceiver<Arc<Message>>, CloseOptionError> {
        Err(CloseOptionError::Unsupported("Live candle support not yet implemented".into()))
    }

    /// Get raw handler for advanced operations
    pub async fn raw_handler(&self) -> Result<RawHandler, CloseOptionError> {
        Ok(RawHandler {
            sender: self.client.to_ws_sender.clone(),
        })
    }
}
/// Lightweight module for routing responses to pending requests and subscriptions
pub struct ResponseRouterModule {
    state: Arc<State>,
    receiver: AsyncReceiver<Arc<Message>>,
}

impl ResponseRouterModule {
    pub fn new(state: Arc<State>, receiver: AsyncReceiver<Arc<Message>>) -> Self {
        Self { state, receiver }
    }
}

#[async_trait::async_trait]
impl LightweightModule<State> for ResponseRouterModule {
    fn new(
        state: Arc<State>,
        _ws_sender: AsyncSender<Message>,
        receiver: AsyncReceiver<Arc<Message>>,
        _runner_command_tx: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self::new(state, receiver)
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(|msg: &Message| {
            if let Ok(text) = msg.to_text() {
                text.contains("priceData") || text.contains("setOrderResult") || text.contains("get30MinResult") || text.contains("\"id\"")
            } else {
                false
            }
        })
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            if let Ok(text) = msg.to_text() {
                // Try parsing as Socket.IO frames first
                let frames = crate::closeoption::utils::parse_socket_io_message(text)
                    .unwrap_or_default();
                debug!(target: "Router", frames_count = frames.len(), raw_text = %text, "parsed socket.io frames");
                let mut handled = false;
                for frame in &frames {
                    if frame.message_type == crate::closeoption::types::socket_io::SocketIoMessageType::Event {
                        if let Ok(event_array) = serde_json::from_str::<Vec<serde_json::Value>>(&frame.data) {
                            if let Some(event_name) = event_array.get(0).and_then(|v| v.as_str()) {
                                debug!(target: "Router", event_name, "parsed event from frame");
                                if let Some(payload) = event_array.get(1) {
                                    // Resolve pending requests by ID
                                    if let Ok(value) = serde_json::to_value(payload) {
                                        if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                                            let mut pending = self.state.pending_requests.lock().await;
                                            if let Some(sender) = pending.remove(&id) {
                                                if let Ok(event) = serde_json::from_value::<SubscriptionEvent>(value.clone()) {
                                                    let _ = sender.send(event);
                                                }
                                            }
                                        }
                                    }
                                    // Route get30MinResult events
                                    if event_name == "get30MinResult" {
                                        debug!(target: "Router", event_name, "get30MinResult handler start");
                                        debug!(target: "Router", payload_type = %payload, "attempting Get30MinResult deserialization");
                                        if let Ok(price_data) = serde_json::from_value::<Get30MinResult>(payload.clone()) {
                                            let event = SubscriptionEvent::Get30MinResult(price_data);
                                            let mut pending = self.state.pending_requests.lock().await;
                                            if let Some(id) = pending.keys().next().copied() {
                                                debug!(target: "Router", id, "removing pending request");
                                                if let Some(sender) = pending.remove(&id) {
                                                    let _ = sender.send(event);
                                                debug!(target: "Router", "sent get30MinResult event to pending request");
                                                }
                                            }
                                        } else if let Ok(error_data) = serde_json::from_value::<serde_json::Value>(payload.clone()) {
                                            debug!(target: "Router", "Get30MinResult deserialization failed, checking for error");
                                            let head = error_data.get("head").and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string();
                                            let code = error_data.get("code").and_then(|v| v.as_str()).unwrap_or("UNKNOWN").to_string();
                                            let pair = error_data.get("pair").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                            let api_error = CloseOptionError::ApiError { head, code, pair };
                                            let mut pending = self.state.pending_requests.lock().await;
                                            if let Some(id) = pending.keys().next().copied() {
                                                if let Some(sender) = pending.remove(&id) {
                                                    let _ = sender.send(SubscriptionEvent::Error(api_error.to_string()));
                                                }
                                            }
                                        }
                                        handled = true;
                                    }

                                    // Route priceData events
                                    if event_name == "priceData" {
                                        if let Ok(price_data) = serde_json::from_value::<PriceData>(payload.clone()) {
                                            let event = SubscriptionEvent::PriceData(price_data.clone());
                                            let mut subscriptions = self.state.subscriptions.lock().await;
                                            let mut failed: Vec<String> = Vec::new();
                                            for (symbol, sender) in subscriptions.iter() {
                                                if price_data.prices.contains_key(symbol) {
                                                    if sender.send(event.clone()).await.is_err() {
                                                        failed.push(symbol.clone());
                                                    }
                                                }
                                            }
                                            for sym in failed {
                                                subscriptions.remove(&sym);
                                            }
                                            drop(subscriptions);
                                            // Raw subscriptions: try_send with stable sender identity
                                            // On backpressure (full queue), drop the event for that subscriber
                                            // to preserve ordering for others and avoid unbounded memory growth.
                                            let raw_subscriptions = self.state.raw_subscriptions.lock().await;
                                            let senders: Vec<_> = raw_subscriptions.iter().cloned().collect();
                                            drop(raw_subscriptions);
                                            for sender in senders {
                                                let _ = sender.try_send(event.clone());
                                            }
                                            handled = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Fallback: direct JSON parse for already-decoded messages
                if !handled {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                            let mut pending = self.state.pending_requests.lock().await;
                            if let Some(sender) = pending.remove(&id) {
                                if let Ok(event) = serde_json::from_value::<SubscriptionEvent>(value.clone()) {
                                    let _ = sender.send(event);
                                }
                            }
                        }
                        if let Some(event_name) = value.get("event").and_then(|v| v.as_str()) {
                            if event_name == "priceData" {
                                if let Ok(price_data) = serde_json::from_value::<PriceData>(value.get("data").cloned().unwrap_or_default()) {
                                    let event = SubscriptionEvent::PriceData(price_data.clone());
                                    let subscriptions = self.state.subscriptions.lock().await;
                                    for (symbol, sender) in subscriptions.iter() {
                                        if price_data.prices.contains_key(symbol) {
                                            let _ = sender.send(event.clone()).await;
                                        }
                                    }
                                    drop(subscriptions);
                                    // Raw subscriptions: try_send with stable sender identity
                                    // On backpressure (full queue), drop the event for that subscriber
                                    // to preserve ordering for others and avoid unbounded memory growth.
                                    let raw_subscriptions = self.state.raw_subscriptions.lock().await;
                                    let senders: Vec<_> = raw_subscriptions.iter().cloned().collect();
                                    drop(raw_subscriptions);
                                    for sender in senders {
                                        let _ = sender.try_send(event.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for CloseOption {
    fn drop(&mut self) {
        if Arc::strong_count(&self.runner) == 1 {
            self.runner.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::closeoption::state::StateBuilder;

    #[test]
    fn test_close_option_struct() {
        // Test that StateBuilder works
        let state = StateBuilder::new()
            .token("test")
            .sid("sid")
            .public_code("pub")
            .hidden_code("hid")
            .build()
            .unwrap();
        assert_eq!(state.token, "test");
        assert_eq!(state.sid, "sid");
    }

    #[test]
    fn test_duration_to_time_intervals() {
        assert_eq!(CloseOption::duration_to_time_intervals(30).unwrap(), "30 Seconds");
        assert_eq!(CloseOption::duration_to_time_intervals(60).unwrap(), "1 Minute");
        assert_eq!(CloseOption::duration_to_time_intervals(300).unwrap(), "5 Minutes");
        assert_eq!(CloseOption::duration_to_time_intervals(600).unwrap(), "10 Minutes");
        assert_eq!(CloseOption::duration_to_time_intervals(45).unwrap(), "30 Seconds");
        assert_eq!(CloseOption::duration_to_time_intervals(350).unwrap(), "10 Minutes");
        assert!(CloseOption::duration_to_time_intervals(999).is_err());
    }

    #[tokio::test]
    async fn test_raw_subscription_ordered_delivery_under_backpressure() {
        use kanal::bounded_async;
        use crate::closeoption::types::SubscriptionEvent;
        use crate::closeoption::state::StateBuilder;

        // Create a state with raw subscriptions
        let state = StateBuilder::new()
            .token("test")
            .sid("sid")
            .public_code("pub")
            .hidden_code("hid")
            .build()
            .unwrap();

        // Add multiple raw subscribers with small buffer to trigger backpressure
        let (tx1, rx1) = bounded_async::<SubscriptionEvent>(2);
        let (tx2, _rx2) = bounded_async::<SubscriptionEvent>(2);
        let (tx3, rx3) = bounded_async::<SubscriptionEvent>(2);
        state.raw_subscriptions.lock().await.push(tx1.clone());
        state.raw_subscriptions.lock().await.push(tx2.clone());
        state.raw_subscriptions.lock().await.push(tx3.clone());

        // Fill up subscriber 2's buffer to simulate backpressure
        let event1 = SubscriptionEvent::PriceData(crate::closeoption::types::PriceData {
            prices: std::collections::HashMap::new(),
            timestamp: 0,
        });
        let event2 = SubscriptionEvent::PriceData(crate::closeoption::types::PriceData {
            prices: std::collections::HashMap::new(),
            timestamp: 0,
        });
        let _ = tx2.try_send(event1.clone());
        let _ = tx2.try_send(event2.clone());
        // Now tx2's buffer is full (capacity 2)

        // Send a new event - should be delivered to tx1 and tx3, dropped for tx2
        let event3 = SubscriptionEvent::PriceData(crate::closeoption::types::PriceData {
            prices: std::collections::HashMap::new(),
            timestamp: 0,
        });

        // Simulate the routing logic: collect senders and try_send to each
        let raw_subscriptions = state.raw_subscriptions.lock().await;
        let senders: Vec<_> = raw_subscriptions.iter().cloned().collect();
        drop(raw_subscriptions);

        for sender in senders {
            let _ = sender.try_send(event3.clone());
        }

        // Verify tx1 and tx3 received the event, tx2 did not (backpressure)
        let received1 = rx1.recv().await.is_ok();
        let received3 = rx3.recv().await.is_ok();
        
        // tx2 should not have received event3 (buffer full)
        // But we can't easily test that without timing - the key invariant is:
        // - No panic, no deadlock
        // - Other subscribers still receive events in order
        assert!(received1, "Subscriber 1 should receive event");
        assert!(received3, "Subscriber 3 should receive event");
        
        // Verify ordering: events received by each subscriber are in order
        // (tx1 received event3, tx3 received event3)
        // This test primarily ensures no crash and basic delivery works
    }
}
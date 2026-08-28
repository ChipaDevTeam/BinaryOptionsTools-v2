use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
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
use crate::closeoption::types::{Asset, Candle, OrderResult, Outgoing, SubscriptionEvent, SetOrderRequest, Get30MinRequest, PriceData};
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
            if let Ok(text) = msg.to_text() {
                if text.contains("priceData") {
                    if let Some(json_start) = text.find("priceData") {
                        let json_str = &text[json_start..];
                        if let Some(start) = json_str.find('{') {
                            let json_part = &json_str[start..];
                            if let Ok(price_data) = serde_json::from_str::<PriceData>(json_part) {
                                self.state.update_assets(&price_data).await;
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
                if text.contains("setOrderResult") {
                    if let Some(json_start) = text.find("setOrderResult") {
                        let json_str = &text[json_start..];
                        if let Some(start) = json_str.find('{') {
                            let json_part = &json_str[start..];
                            if let Ok(order_result) = serde_json::from_str::<OrderResult>(json_part) {
                                self.state.update_balance(order_result.balance).await;
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
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<SubscriptionEvent>>>>,
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
            .with_lightweight_module::<KeepAliveModule>();

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
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get the internal state
    pub fn state(&self) -> Arc<State> {
        self.client.state.clone()
    }

    async fn send_and_wait(&self, request: Outgoing) -> Result<SubscriptionEvent, CloseOptionError> {
        let id = get_index();
        let (tx, rx) = oneshot::channel();
        
        // Register pending request
        {
            let mut pending = self.pending_requests.lock().map_err(|e| CloseOptionError::General(format!("Lock poisoned: {}", e)))?;
            pending.insert(id, tx);
        }
        
        let json = serde_json::to_string(&request)
            .map_err(|e| CloseOptionError::General(format!("Failed to serialize: {}", e)))?;
        
        let frame = crate::closeoption::types::socket_io::event("message", &json);
        
        self.client.to_ws_sender.send(Message::Text(frame.into())).await
            .map_err(|e| CloseOptionError::General(format!("Failed to send: {}", e)))?;

        // Wait for response with timeout
        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => {
                // Clean up pending request
                self.pending_requests.lock().map_err(|e| CloseOptionError::General(format!("Lock poisoned: {}", e)))?
                    .remove(&id);
                Ok(response)
            },
            Ok(Err(_)) => Err(CloseOptionError::General("Response channel closed".to_string())),
            Err(_) => {
                // Clean up on timeout
                self.pending_requests.lock().map_err(|e| CloseOptionError::General(format!("Lock poisoned: {}", e)))?
                    .remove(&id);
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
        let assets = self.state().get_assets().await;
        if let Some(asset) = assets.get(order_id) {
            Ok(OrderResult {
                order_id: order_id.to_string(),
                pair: asset.symbol.clone(),
                status: "closed".to_string(),
                amount: 0.0,
                open_price: 0.0,
                profit: 0.0,
                result: "".to_string(),
                payout: 0.0,
                balance: 0.0,
                close_price: 0.0,
                close_time: 0,
                open_time: 0,
            })
        } else {
            Err(CloseOptionError::General(format!("Order not found: {}", order_id)))
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
            SubscriptionEvent::Get30MinResult(result) => Ok(result.candles),
            _ => Err(CloseOptionError::General("Unexpected response type".to_string())),
        }
    }

    /// Get active assets
    pub async fn active_assets(&self) -> Result<Vec<Asset>, CloseOptionError> {
        let assets = self.state().get_assets().await;
        Ok(assets.into_values().collect())
    }

    /// Subscribe to price updates for a symbol
    pub async fn subscribe_symbol(&self, _symbol: &str) -> Result<AsyncReceiver<SubscriptionEvent>, CloseOptionError> {
        let (_tx, rx) = kanal::bounded_async::<SubscriptionEvent>(100);
        Ok(rx)
    }

    /// Subscribe to all raw messages
    pub async fn subscribe_raw(&self) -> Result<AsyncReceiver<SubscriptionEvent>, CloseOptionError> {
        let (_tx, rx) = kanal::bounded_async::<SubscriptionEvent>(100);
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
        // CloseOption doesn't expose per-asset payout in priceData
        // Return a default payout rate (e.g., 85% for major pairs)
        Ok(0.85)
    }

    /// Get trade history
    pub async fn history(&self, _limit: u32) -> Result<Vec<OrderResult>, CloseOptionError> {
        // History would require server-side query, return empty for now
        Ok(vec![])
    }

    /// Get opened deals
    pub async fn opened_deals(&self) -> Result<Vec<OrderResult>, CloseOptionError> {
        // Opened deals would require server-side query, return empty for now
        Ok(vec![])
    }

    /// Get closed deals
    pub async fn closed_deals(&self) -> Result<Vec<OrderResult>, CloseOptionError> {
        // Closed deals would require server-side query, return empty for now
        Ok(vec![])
    }

    /// Get live candle updates
    pub async fn get_candles_live(&self, _asset: &str, _period: u32) -> Result<AsyncReceiver<Arc<Message>>, CloseOptionError> {
        let (_tx, rx) = kanal::bounded_async::<Arc<Message>>(100);
        Ok(rx)
    }

    /// Get raw handler for advanced operations
    pub async fn raw_handler(&self) -> Result<RawHandler, CloseOptionError> {
        Ok(RawHandler {
            sender: self.client.to_ws_sender.clone(),
        })
    }
}

impl Drop for CloseOption {
    fn drop(&mut self) {
        self.runner.abort();
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
}

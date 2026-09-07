use kanal;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use binary_options_tools_core::traits::AppState;

use crate::closeoption::error::CloseOptionError;
use crate::closeoption::types::{Asset, OrderResult, PriceData, SubscriptionEvent};
use crate::closeoption::utils::normalize_timestamp;
/// Application state for CloseOption client
///
/// This structure holds all the shared state for the CloseOption client,
/// including session information, connection settings, and real-time data
/// like balance and assets.
///
/// # Thread Safety
///
/// All fields are designed to be thread-safe, allowing concurrent access
/// from multiple modules and tasks.
#[derive(Debug, Clone)]
pub struct State {
    /// Authentication token
    pub token: String,
    /// Session ID from Socket.IO handshake
    pub sid: String,
    /// Whether this is a demo account
    pub is_demo: bool,
    /// Public code from session
    pub public_code: String,
    /// Hidden code from session
    pub hidden_code: String,
    /// Current balance (updated from order results)
    pub balance: Arc<RwLock<Option<f64>>>,
    /// Available assets
    pub assets: Arc<RwLock<HashMap<String, Asset>>>,
    /// Server time offset (server_time - local_time) in seconds
    pub server_time_offset: Arc<RwLock<i64>>,
    /// User agent for WebSocket connection
    pub user_agent: Option<String>,
    /// Origin header
    pub origin: Option<String>,
    /// Proxy URL
    pub proxy: Option<String>,
    /// TLS cipher suites
    pub tls_cipher_suites: Option<Vec<String>>,
    /// TLS ALPN protocols
    pub tls_alpn: Option<Vec<String>>,
    /// Sec-WebSocket-Extensions
    pub sec_websocket_extensions: Option<String>,
    /// Pending request-response channels keyed by request ID (ordered for FIFO fallback)
    pub pending_requests: Arc<Mutex<BTreeMap<u64, oneshot::Sender<SubscriptionEvent>>>>,
    /// Symbol subscriptions: symbol -> sender
    pub subscriptions: Arc<Mutex<HashMap<String, kanal::AsyncSender<SubscriptionEvent>>>>,
    /// Raw subscriptions: all events broadcast to all senders
    pub raw_subscriptions: Arc<Mutex<Vec<kanal::AsyncSender<SubscriptionEvent>>>>,
    /// Order results keyed by order ID
    pub orders: Arc<Mutex<HashMap<String, OrderResult>>>,
    /// Custom WebSocket URL (if set, ws_url() returns this instead of the default)
    pub ws_url: Option<String>,
}

/// Builder pattern for creating State instances
///
/// This builder provides a fluent interface for constructing State objects
/// with proper validation and defaults.
#[derive(Default)]
pub struct StateBuilder {
    token: Option<String>,
    sid: Option<String>,
    is_demo: bool,
    public_code: Option<String>,
    hidden_code: Option<String>,
    user_agent: Option<String>,
    origin: Option<String>,
    proxy: Option<String>,
    tls_cipher_suites: Option<Vec<String>>,
    tls_alpn: Option<Vec<String>>,
    sec_websocket_extensions: Option<String>,
    ws_url: Option<String>,
}

impl StateBuilder {
    /// Create a new StateBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the authentication token (required)
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the session ID (required)
    pub fn sid(mut self, sid: impl Into<String>) -> Self {
        self.sid = Some(sid.into());
        self
    }

    /// Set demo mode (default: false)
    pub fn demo(mut self, is_demo: bool) -> Self {
        self.is_demo = is_demo;
        self
    }

    /// Set public code (required)
    pub fn public_code(mut self, code: impl Into<String>) -> Self {
        self.public_code = Some(code.into());
        self
    }

    /// Set hidden code (required)
    pub fn hidden_code(mut self, code: impl Into<String>) -> Self {
        self.hidden_code = Some(code.into());
        self
    }

    /// Set custom user agent
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Set custom origin
    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }

    /// Set proxy URL
    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.proxy = Some(proxy.into());
        self
    }

    /// Set TLS cipher suites
    pub fn tls_cipher_suites(mut self, suites: Vec<String>) -> Self {
        self.tls_cipher_suites = Some(suites);
        self
    }

    /// Set TLS ALPN protocols
    pub fn tls_alpn(mut self, alpn: Vec<String>) -> Self {
        self.tls_alpn = Some(alpn);
        self
    }

    /// Set Sec-WebSocket-Extensions
    pub fn sec_websocket_extensions(mut self, ext: impl Into<String>) -> Self {
        self.sec_websocket_extensions = Some(ext.into());
        self
    }

    /// Set custom WebSocket URL
    pub fn ws_url(mut self, url: impl Into<String>) -> Self {
        self.ws_url = Some(url.into());
        self
    }

    /// Build the State, validating required fields
    pub fn build(self) -> Result<State, CloseOptionError> {
        let token = self
            .token
            .ok_or_else(|| CloseOptionError::StateBuilder("token is required".to_string()))?;
        let sid = self
            .sid
            .ok_or_else(|| CloseOptionError::StateBuilder("sid is required".to_string()))?;
        let public_code = self
            .public_code
            .ok_or_else(|| CloseOptionError::StateBuilder("public_code is required".to_string()))?;
        let hidden_code = self
            .hidden_code
            .ok_or_else(|| CloseOptionError::StateBuilder("hidden_code is required".to_string()))?;

        Ok(State {
            token,
            sid,
            is_demo: self.is_demo,
            public_code,
            hidden_code,
            balance: Arc::new(RwLock::new(None)),
            assets: Arc::new(RwLock::new(HashMap::new())),
            server_time_offset: Arc::new(RwLock::new(0)),
            user_agent: self.user_agent,
            origin: self.origin,
            proxy: self.proxy,
            tls_cipher_suites: self.tls_cipher_suites,
            tls_alpn: self.tls_alpn,
            sec_websocket_extensions: self.sec_websocket_extensions,
            pending_requests: Arc::new(Mutex::new(BTreeMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            raw_subscriptions: Arc::new(Mutex::new(Vec::new())),
            orders: Arc::new(Mutex::new(HashMap::new())),
            ws_url: self.ws_url,
        })
    }
}

impl State {
    /// Create a new StateBuilder
    pub fn builder() -> StateBuilder {
        StateBuilder::new()
    }

    /// Get the account type string for API requests
    pub fn acc_type(&self) -> &str {
        if self.is_demo {
            "demo"
        } else {
            "real"
        }
    }

    /// Get the base WebSocket URL with session ID
    pub fn ws_url(&self) -> String {
        match &self.ws_url {
            Some(url) => url.clone(),
            None => format!(
                "wss://www.closeoption.com:8443/socket.io/?EIO=3&transport=websocket&sid={}",
                self.sid
            ),
        }
    }

    /// Update balance from order result
    pub async fn update_balance(&self, balance: f64) {
        let mut b = self.balance.write().await;
        *b = Some(balance);
    }

    /// Get current balance
    pub async fn get_balance(&self) -> Option<f64> {
        *self.balance.read().await
    }

    /// Update assets from price data
    pub async fn update_assets(&self, price_data: &PriceData) {
        let mut assets = self.assets.write().await;
        for (symbol, price) in &price_data.prices {
            let asset = Asset {
                symbol: symbol.clone(),
                bid: price.bid,
                ask: price.ask,
                main: price.main,
                source: "AFX".to_string(),
            };
            assets.insert(symbol.clone(), asset);
        }
    }

    /// Get all assets
    pub async fn get_assets(&self) -> HashMap<String, Asset> {
        self.assets.read().await.clone()
    }

    /// Get asset by symbol
    pub async fn get_asset(&self, symbol: &str) -> Option<Asset> {
        self.assets.read().await.get(symbol).cloned()
    }

    /// Update server time offset
    pub async fn update_server_time_offset(&self, server_time: i64) {
        let local_time = chrono::Utc::now().timestamp();
        let mut offset = self.server_time_offset.write().await;
        *offset = server_time - local_time;
    }

    /// Get server time offset
    pub async fn get_server_time_offset(&self) -> i64 {
        *self.server_time_offset.read().await
    }

    /// Get current server time (local time + offset)
    pub async fn server_time(&self) -> i64 {
        let offset = self.get_server_time_offset().await;
        chrono::Utc::now().timestamp() + offset
    }

    /// Normalize a timestamp using server time offset
    pub async fn normalize_timestamp(&self, raw: f64) -> i64 {
        let normalized = normalize_timestamp(raw);
        let offset = self.get_server_time_offset().await;
        if offset != 0 {
            normalized + offset
        } else {
            normalized
        }
    }

    /// Clear temporal data (called on disconnect)
    pub async fn clear_temporal_data(&self) {
        *self.balance.write().await = None;
        self.assets.write().await.clear();
        self.orders.lock().await.clear();
        *self.server_time_offset.write().await = 0;
    }
}
#[async_trait::async_trait]
impl AppState for State {
    async fn clear_temporal_data(&self) {
        self.clear_temporal_data().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_builder() {
        let state = StateBuilder::new()
            .token("test")
            .sid("sid")
            .public_code("pub")
            .hidden_code("hid")
            .build()
            .unwrap();
        assert_eq!(state.token, "test");
        assert_eq!(state.sid, "sid");
        assert_eq!(
            state.ws_url(),
            "wss://www.closeoption.com:8443/socket.io/?EIO=3&transport=websocket&sid=sid"
        );
    }

    #[test]
    fn test_state_builder_with_custom_url() {
        let state = StateBuilder::new()
            .token("test")
            .sid("sid")
            .public_code("pub")
            .hidden_code("hid")
            .ws_url("wss://custom.example.com/socket")
            .build()
            .unwrap();
        assert_eq!(state.ws_url(), "wss://custom.example.com/socket");
    }
}

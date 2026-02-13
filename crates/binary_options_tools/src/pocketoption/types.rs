use core::fmt;
use std::hash::Hash;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

use binary_options_tools_core_pre::{reimports::Message, traits::Rule};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;

use crate::pocketoption::error::{PocketError, PocketResult};

// 🚨 CRITICAL AUDIT NOTE:
// Financial values (amount, price, profit) are currently represented as `f64`.
// This can lead to floating-point precision errors in financial calculations.
// While the upstream PocketOption API uses JSON numbers (which are often treated as floats),
// best practice would be to use `rust_decimal::Decimal`.
// Migration to `Decimal` is recommended for future versions but requires updating
// the Python bindings and verifying JSON serialization compatibility.

/// Server time management structure for synchronizing with PocketOption servers
///
/// This structure maintains the relationship between server time and local time,
/// allowing for accurate time synchronization across different time zones and
/// network delays.
#[derive(Debug, Clone)]
pub struct ServerTime {
    /// Last received server timestamp (Unix timestamp as i64)
    pub last_server_time: i64,
    /// Local time when the server time was last updated
    pub last_updated: DateTime<Utc>,
    /// Calculated offset between server time and local time
    pub offset: Duration,
}

impl Default for ServerTime {
    fn default() -> Self {
        Self {
            last_server_time: 0,
            last_updated: Utc::now(),
            offset: Duration::zero(),
        }
    }
}

impl ServerTime {
    /// Update server time with a new timestamp from the server
    ///
    /// This method calculates the offset between server time and local time
    /// to maintain accurate synchronization.
    ///
    /// # Arguments
    /// * `server_timestamp` - Unix timestamp from the server as i64
    pub fn update(&mut self, server_timestamp: i64) {
        let now = Utc::now();
        let local_timestamp = now.timestamp();

        self.last_server_time = server_timestamp;
        self.last_updated = now;

        // Calculate offset: server time - local time
        let offset_seconds = server_timestamp - local_timestamp;
        self.offset = Duration::seconds(offset_seconds);
    }

    /// Convert local time to estimated server time
    ///
    /// # Arguments
    /// * `local_time` - Local `DateTime<Utc>` to convert
    ///
    /// # Returns
    /// Estimated server timestamp as i64
    pub fn local_to_server(&self, local_time: DateTime<Utc>) -> i64 {
        let local_timestamp = local_time.timestamp();
        local_timestamp + self.offset.num_seconds()
    }

    /// Convert server time to local time
    ///
    /// # Arguments
    /// * `server_timestamp` - Server timestamp as i64
    ///
    /// # Returns
    /// Local `DateTime<Utc>`
    pub fn server_to_local(&self, server_timestamp: i64) -> DateTime<Utc> {
        let adjusted = server_timestamp - self.offset.num_seconds();
        DateTime::from_timestamp(adjusted.max(0), 0).unwrap_or_else(Utc::now)
    }

    /// Get current estimated server time
    ///
    /// # Returns
    /// Current estimated server timestamp as i64
    pub fn get_server_time(&self) -> i64 {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_updated);
        self.last_server_time + elapsed.num_seconds()
    }

    /// Check if the server time data is stale (older than 30 seconds)
    ///
    /// # Returns
    /// True if the server time data is considered stale
    pub fn is_stale(&self) -> bool {
        let now = Utc::now();
        now.signed_duration_since(self.last_updated) > Duration::seconds(30)
    }
}

impl fmt::Display for ServerTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ServerTime(last_server_time: {}, last_updated: {}, offset: {})",
            self.last_server_time, self.last_updated, self.offset
        )
    }
}

/// Stream data from WebSocket messages
///
/// This represents the raw price data received from PocketOption's WebSocket API
/// in the format: [["SYMBOL",timestamp,price]]
#[derive(Debug, Clone)]
pub struct StreamData {
    /// Trading symbol (e.g., "EURUSD_otc")
    pub symbol: String,
    /// Unix timestamp from server
    pub timestamp: i64,
    /// Current price
    pub price: Decimal,
}

/// Implement the custom deserialization for StreamData
/// This allows StreamData to be deserialized from the WebSocket message format
impl<'de> Deserialize<'de> for StreamData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec: Vec<Vec<serde_json::Value>> = Vec::deserialize(deserializer)?;
        if vec.len() != 1 {
            return Err(serde::de::Error::custom("Invalid StreamData format"));
        }
        if vec[0].len() != 3 {
            return Err(serde::de::Error::custom("Invalid StreamData format"));
        }

        let price_f64 = vec[0][2].as_f64().unwrap_or(0.0);
        let price = Decimal::from_f64_retain(price_f64).unwrap_or_default();

        Ok(StreamData {
            symbol: vec[0][0].as_str().unwrap_or_default().to_string(),
            timestamp: vec[0][1].as_f64().unwrap_or(0.0) as i64,
            price,
        })
    }
}

impl StreamData {
    /// Create new stream data
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol
    /// * `timestamp` - Unix timestamp
    /// * `price` - Current price
    pub fn new(symbol: String, timestamp: i64, price: Decimal) -> Self {
        Self {
            symbol,
            timestamp,
            price,
        }
    }

    /// Convert timestamp to `DateTime<Utc>`
    ///
    /// # Returns
    /// `DateTime<Utc>` representation of the timestamp
    pub fn datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp, 0).unwrap_or_else(Utc::now)
    }
}

/// Type alias for thread-safe server time state
///
/// This provides shared access to server time data across multiple modules
/// using a read-write lock for concurrent access.
pub type ServerTimeState = tokio::sync::RwLock<ServerTime>;

/// Simple rule implementation for when the websocket data is sent using 2 messages
/// The first one telling which message type it is, and the second one containing the actual data.
pub struct TwoStepRule {
    valid: AtomicBool,
    pattern: String,
}

impl TwoStepRule {
    /// Create a new TwoStepRule with the specified pattern
    ///
    /// # Arguments
    /// * `pattern` - The string pattern to match against incoming messages
    pub fn new(pattern: impl ToString) -> Self {
        Self {
            valid: AtomicBool::new(false),
            pattern: pattern.to_string(),
        }
    }
}

impl Rule for TwoStepRule {
    fn call(&self, msg: &Message) -> bool {
        tracing::debug!(target: "TwoStepRule", "Checking message against pattern '{}': {:?}", self.pattern, msg);
        match msg {
            Message::Text(text) => {
                if text.starts_with(&self.pattern) {
                    tracing::debug!(target: "TwoStepRule", "Pattern matched! Next message will be accepted.");
                    self.valid.store(true, Ordering::SeqCst);
                    return false;
                }

                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    return true;
                }
                false
            }
            Message::Binary(_) => {
                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn reset(&self) {
        self.valid.store(false, Ordering::SeqCst)
    }
}

/// More advanced implementation of the TwoStepRule that allows for multipple patterns
///
/// **Message Routing with `MultiPatternRule`:**
/// This rule is designed to process Socket.IO messages that follow a common pattern
/// for event-based communication. It expects incoming `Message::Text` to be a JSON
/// array where the first element is a string representing the logical event name.
///
/// - **Patterns:** The `patterns` provided to `MultiPatternRule::new` should be the
///   *exact logical event names* (e.g., `"updateHistory"`, `"successOpenOrder"`).
/// - **Framing:** Do *not* include any numeric prefixes (like `42` or `451-`) or other
///   Socket.IO framing characters in the patterns. These will be automatically handled
///   by the rule's parsing logic.
/// - **Behavior:** When a `Message::Text` containing a matching event name is received,
///   the rule internally flags `valid` as true. The *next* `Message::Binary` received
///   after this flag is set will be considered part of the two-step message and allowed
///   to pass through (by returning `true` from `call`). All other messages will be filtered.
pub struct MultiPatternRule {
    valid: AtomicBool,
    patterns: Vec<String>,
}

impl MultiPatternRule {
    /// Create a new MultiPatternRule with the specified patterns
    ///
    /// # Arguments
    /// * `patterns` - The string patterns to match against incoming messages
    pub fn new(patterns: Vec<impl ToString>) -> Self {
        Self {
            valid: AtomicBool::new(false),
            patterns: patterns.into_iter().map(|p| p.to_string()).collect(),
        }
    }
}

impl Rule for MultiPatternRule {
    fn call(&self, msg: &Message) -> bool {
        match msg {
            Message::Text(text) => {
                if let Some(start) = text.find('[') {
                    if let Ok(value) = serde_json::from_str::<Value>(&text[start..]) {
                        if let Some(arr) = value.as_array() {
                            if let Some(event_name) = arr.first().and_then(|v| v.as_str()) {
                                for pattern in &self.patterns {
                                    if event_name == pattern {
                                        // Detect if this is a binary placeholder
                                        let has_placeholder = arr.iter().skip(1).any(|v| {
                                            v.as_object()
                                                .is_some_and(|obj| obj.contains_key("_placeholder"))
                                        });

                                        if arr.len() == 1 || has_placeholder {
                                            self.valid.store(true, Ordering::SeqCst);
                                            return false;
                                        } else {
                                            // 1-step message, allow it through
                                            self.valid.store(false, Ordering::SeqCst);
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    return true;
                }
                false
            }
            Message::Binary(_) => {
                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn reset(&self) {
        self.valid.store(false, Ordering::SeqCst)
    }
}

/// CandleLength is a wrapper around u32 for allowed candle durations (in seconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CandleLength {
    time: u32,
}

impl CandleLength {
    /// Create a new CandleLength instance
    ///
    /// # Arguments
    /// * `time` - Duration in seconds
    pub const fn new(time: u32) -> Self {
        CandleLength { time }
    }

    /// Get the duration in seconds
    pub fn duration(&self) -> u32 {
        self.time
    }
}

impl From<u32> for CandleLength {
    fn from(val: u32) -> Self {
        CandleLength { time: val }
    }
}
impl From<CandleLength> for u32 {
    fn from(val: CandleLength) -> u32 {
        val.time
    }
}

/// Asset struct for processed asset data
#[derive(Debug, Clone, Serialize)]
pub struct Asset {
    pub id: i32, // This field is not used in the current implementation but can be useful for debugging
    pub name: String,
    pub symbol: String,
    pub is_otc: bool,
    pub is_active: bool,
    pub payout: i32,
    pub allowed_candles: Vec<CandleLength>,
    pub asset_type: AssetType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    Stock,
    Currency,
    Commodity,
    Cryptocurrency,
    Index,
}

impl Asset {
    pub fn is_otc(&self) -> bool {
        self.is_otc
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn allowed_candles(&self) -> &[CandleLength] {
        &self.allowed_candles
    }

    /// Validates if the asset can be used for trading
    /// It checks if the asset is active.
    /// The error thrown allows users to understand why the asset is not valid for trading.
    ///
    /// Note: Time validation has been removed to allow trading at any expiration time.
    pub fn validate(&self, time: u32) -> PocketResult<()> {
        if !self.is_active {
            return Err(PocketError::InvalidAsset("Asset is not active".into()));
        }
        if 24 * 60 * 60 % time != 0 {
            return Err(PocketError::InvalidAsset(
                "Time must be a divisor of 86400 (24 hours)".into(),
            ));
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Asset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[allow(dead_code)] // Allow dead code because many fields are unused but kept for wire compatibility
        struct AssetRawTuple(
            i32,                   // 0: id (used)
            String,                // 1: symbol (used)
            String,                // 2: name (used)
            AssetType,             // 3: asset_type (used)
            serde::de::IgnoredAny, // 4: unused
            i32,                   // 5: payout (used)
            serde::de::IgnoredAny, // 6: unused
            serde::de::IgnoredAny, // 7: unused
            serde::de::IgnoredAny, // 8: unused
            i32,                   // 9: is_otc (used, 1 for true, 0 for false)
            serde::de::IgnoredAny, // 10: unused
            serde::de::IgnoredAny, // 11: unused
            serde::de::IgnoredAny, // 12: unused
            serde::de::IgnoredAny, // 13: unused
            bool,                  // 14: is_active (used)
            Vec<CandleLength>,     // 15: allowed_candles (used)
            serde::de::IgnoredAny, // 16: unused
            serde::de::IgnoredAny, // 17: unused
            serde::de::IgnoredAny, // 18: unused
        );

        let raw: AssetRawTuple = AssetRawTuple::deserialize(deserializer)?;
        Ok(Asset {
            id: raw.0,
            symbol: raw.1,
            name: raw.2,
            asset_type: raw.3,
            payout: raw.5,
            is_otc: raw.9 == 1,
            is_active: raw.14,
            allowed_candles: raw.15,
        })
    }
}

/// Wrapper around HashMap<String, Asset>
#[derive(Debug, Default, Clone, Serialize)]
pub struct Assets(pub HashMap<String, Asset>);

impl Assets {
    pub fn get(&self, symbol: &str) -> Option<&Asset> {
        self.0.get(symbol)
    }

    pub fn validate(&self, symbol: &str, time: u32) -> PocketResult<()> {
        if let Some(asset) = self.get(symbol) {
            asset.validate(time)
        } else {
            Err(PocketError::InvalidAsset(format!(
                "Asset with symbol `{symbol}` not found"
            )))
        }
    }

    pub fn names(&self) -> Vec<&str> {
        self.0.values().map(|a| a.name.as_str()).collect()
    }

    pub fn active_count(&self) -> usize {
        self.0.values().filter(|a| a.is_active).count()
    }

    pub fn active_iter(&self) -> impl Iterator<Item = &Asset> {
        self.0.values().filter(|a| a.is_active)
    }

    pub fn active(&self) -> Self {
        let active = self
            .0
            .iter()
            .filter(|(_, a)| a.is_active)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Assets(active)
    }
}

impl<'de> Deserialize<'de> for Assets {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let assets: Vec<Asset> = Vec::deserialize(deserializer)?;
        let map = assets.into_iter().map(|a| (a.symbol.clone(), a)).collect();
        Ok(Assets(map))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Call, // Buy
    Put,  // Sell
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailOpenOrder {
    pub error: String,
    pub amount: Decimal,
    pub asset: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrder {
    asset: String,
    action: Action,
    #[serde(with = "rust_decimal::serde::float")]
    amount: Decimal,
    is_demo: u32,
    option_type: u32,
    request_id: Uuid,
    time: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Deal {
    pub id: Uuid,
    pub open_time: String,
    pub close_time: String,
    #[serde(with = "crate::pocketoption::utils::unix_timestamp")]
    pub open_timestamp: DateTime<Utc>,
    #[serde(with = "crate::pocketoption::utils::unix_timestamp")]
    pub close_timestamp: DateTime<Utc>,
    pub refund_time: Option<Value>,
    pub refund_timestamp: Option<Value>,
    pub uid: u64,
    pub request_id: Option<Uuid>,
    pub amount: Decimal,
    pub profit: Decimal,
    pub percent_profit: i32,
    pub percent_loss: i32,
    pub open_price: Decimal,
    pub close_price: Decimal,
    pub command: i32,
    pub asset: String,
    pub is_demo: u32,
    pub copy_ticket: String,
    pub open_ms: i32,
    pub close_ms: Option<i32>,
    pub option_type: i32,
    pub is_rollover: Option<bool>,
    pub is_copy_signal: Option<bool>,
    #[serde(rename = "isAI")]
    pub is_ai: Option<bool>,
    pub currency: String,
    pub amount_usd: Option<Decimal>,
    #[serde(rename = "amountUSD")]
    pub amount_usd2: Option<Decimal>,
}

impl Hash for Deal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.uid.hash(state);
    }
}

impl Eq for Deal {}

impl OpenOrder {
    pub fn new(
        amount: Decimal,
        asset: String,
        action: Action,
        duration: u32,
        demo: u32,
        request_id: Uuid,
    ) -> Self {
        Self {
            amount,
            asset,
            action,
            is_demo: demo,
            option_type: 100, // FIXME: Check why it always is 100
            request_id,
            time: duration,
        }
    }
}

impl std::cmp::PartialEq<Uuid> for Deal {
    fn eq(&self, other: &Uuid) -> bool {
        &self.id == other
    }
}

pub fn serialize_action<S>(action: &Action, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match action {
        Action::Call => 0.serialize(serializer),
        Action::Put => 1.serialize(serializer),
    }
}

impl fmt::Display for OpenOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // returns data in this format (using serde_json): 42["openOrder",{"asset":"EURUSD_otc","amount":1.0,"action":"call","isDemo":1,"requestId":"abcde-12345","optionType":100,"time":60}]
        let data = serde_json::to_string(&self).map_err(|_| fmt::Error)?;
        write!(f, "42[\"openOrder\",{data}]")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingOrder {
    pub ticket: Uuid,
    pub open_type: u32,
    pub amount: Decimal,
    pub symbol: String,
    pub open_time: String,
    pub open_price: Decimal,
    pub timeframe: u32,
    pub min_payout: u32,
    pub command: u32,
    pub date_created: String,
    pub id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenPendingOrder {
    open_type: u32,
    amount: Decimal,
    asset: String,
    open_time: u32,
    open_price: Decimal,
    timeframe: u32,
    min_payout: u32,
    command: u32,
}

impl OpenPendingOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        open_type: u32,
        amount: Decimal,
        asset: String,
        open_time: u32,
        open_price: Decimal,
        timeframe: u32,
        min_payout: u32,
        command: u32,
    ) -> Self {
        Self {
            open_type,
            amount,
            asset,
            open_time,
            open_price,
            timeframe,
            min_payout,
            command,
        }
    }
}

impl fmt::Display for OpenPendingOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let data = serde_json::to_string(&self).map_err(|_| fmt::Error)?;
        write!(f, "42[\"openPendingOrder\",{data}]")
    }
}
#[derive(Debug, Clone)]
pub enum SubscriptionEvent {
    Update {
        asset: String,
        price: Decimal,
        timestamp: i64,
    },
    Terminated {
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub enum Outgoing {
    Text(String),
    Binary(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_data_deserialization() {
        // Test with integer timestamp
        let json_int = r#"[["EURUSD_otc",1770856131,1.19537]]"#;
        let data_int: StreamData = serde_json::from_str(json_int).unwrap();
        assert_eq!(data_int.symbol, "EURUSD_otc");
        assert_eq!(data_int.timestamp, 1770856131);
        assert_eq!(data_int.price, Decimal::from_f64_retain(1.19537).unwrap());

        // Test with float timestamp (the case that was failing)
        let json_float = r#"[["EURUSD_otc",1770856131.3,1.19537]]"#;
        let data_float: StreamData = serde_json::from_str(json_float).unwrap();
        assert_eq!(data_float.symbol, "EURUSD_otc");
        assert_eq!(data_float.timestamp, 1770856131);
        assert_eq!(data_float.price, Decimal::from_f64_retain(1.19537).unwrap());
    }

    #[test]
    fn test_open_order_format() {
        let order = OpenOrder::new(
            Decimal::from_f64_retain(1.0).unwrap(),
            "EURUSD_otc".to_string(),
            Action::Call,
            60,
            1,
            Uuid::new_v4(),
        );
        let formatted = format!("{order}");
        assert!(formatted.starts_with("42[\"openOrder\","));
        assert!(formatted.contains("\"asset\":\"EURUSD_otc\""));
        assert!(formatted.contains("\"amount\":1.0"));
        assert!(formatted.contains("\"action\":\"call\""));
        assert!(formatted.contains("\"isDemo\":1"));
        assert!(formatted.contains("\"optionType\":100"));
        assert!(formatted.contains("\"time\":60"));
        dbg!(formatted);
    }
}

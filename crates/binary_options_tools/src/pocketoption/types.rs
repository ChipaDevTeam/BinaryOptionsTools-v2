use core::fmt;
use std::hash::Hash;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

use binary_options_tools_core_pre::{reimports::Message, traits::Rule};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;

use crate::pocketoption::error::{PocketError, PocketResult};
use crate::pocketoption::utils::float_time;
/// Server time management structure for synchronizing with PocketOption servers
///
/// This structure maintains the relationship between server time and local time,
/// allowing for accurate time synchronization across different time zones and
/// network delays.
#[derive(Debug, Clone)]
pub struct ServerTime {
    /// Last received server timestamp (Unix timestamp as f64)
    pub last_server_time: f64,
    /// Local time when the server time was last updated
    pub last_updated: DateTime<Utc>,
    /// Calculated offset between server time and local time
    pub offset: Duration,
}

impl Default for ServerTime {
    fn default() -> Self {
        Self {
            last_server_time: 0.0,
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
    /// * `server_timestamp` - Unix timestamp from the server as f64
    pub fn update(&mut self, server_timestamp: f64) {
        let now = Utc::now();
        let local_timestamp = now.timestamp() as f64;

        self.last_server_time = server_timestamp;
        self.last_updated = now;

        // Calculate offset: server time - local time
        let offset_seconds = server_timestamp - local_timestamp;
        // Convert to Duration, handling negative values properly
        if offset_seconds >= 0.0 {
            self.offset = Duration::milliseconds((offset_seconds * 1000.0) as i64);
        } else {
            self.offset = Duration::milliseconds(-((offset_seconds.abs() * 1000.0) as i64));
        }
    }

    /// Convert local time to estimated server time
    ///
    /// # Arguments
    /// * `local_time` - Local DateTime<Utc> to convert
    ///
    /// # Returns
    /// Estimated server timestamp as f64
    pub fn local_to_server(&self, local_time: DateTime<Utc>) -> f64 {
        let local_timestamp = local_time.timestamp() as f64;
        local_timestamp + self.offset.num_seconds() as f64
    }

    /// Convert server time to local time
    ///
    /// # Arguments
    /// * `server_timestamp` - Server timestamp as f64
    ///
    /// # Returns
    /// Local DateTime<Utc>
    pub fn server_to_local(&self, server_timestamp: f64) -> DateTime<Utc> {
        let adjusted = server_timestamp - self.offset.num_seconds() as f64;
        DateTime::from_timestamp(adjusted.max(0.0) as i64, 0).unwrap_or_else(Utc::now)
    }

    /// Get current estimated server time
    ///
    /// # Returns
    /// Current estimated server timestamp as f64
    pub fn get_server_time(&self) -> f64 {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_updated);
        self.last_server_time + elapsed.num_seconds() as f64
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
    pub timestamp: f64,
    /// Current price
    pub price: f64,
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
        Ok(StreamData {
            symbol: vec[0][0].as_str().unwrap_or_default().to_string(),
            timestamp: vec[0][1].as_f64().unwrap_or(0.0),
            price: vec[0][2].as_f64().unwrap_or(0.0),
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
    pub fn new(symbol: String, timestamp: f64, price: f64) -> Self {
        Self {
            symbol,
            timestamp,
            price,
        }
    }

    /// Convert timestamp to DateTime<Utc>
    ///
    /// # Returns
    /// DateTime<Utc> representation of the timestamp
    pub fn datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp as i64, 0).unwrap_or_else(Utc::now)
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
        match msg {
            Message::Text(text) => {
                if text.starts_with(&self.pattern) {
                    self.valid.store(true, Ordering::SeqCst);
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
        // tracing::info!("Called with message: {:?}", msg);
        match msg {
            Message::Text(text) => {
                for pattern in &self.patterns {
                    if text.starts_with(pattern) {
                        self.valid.store(true, Ordering::SeqCst);
                        return false;
                    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    Stock,
    Currency,
    Commodity,
    Cryptocurrency,
    Index,
}

impl Asset {
    const DEFAULT_CANDLE_LENGTHS: [CandleLength; 9] = [
        CandleLength::new(5),
        CandleLength::new(15),
        CandleLength::new(30),
        CandleLength::new(60),
        CandleLength::new(60 * 3),
        CandleLength::new(60 * 5),
        CandleLength::new(60 * 30),
        CandleLength::new(60 * 60),
        CandleLength::new(60 * 60 * 4),
    ];

    pub fn is_otc(&self) -> bool {
        self.is_otc
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn allowed_candles(&self) -> &[CandleLength] {
        &self.allowed_candles
    }

    /// Validates if the asset can be used for trading at the given time
    /// It checks, if the time is in the allowed candle durations
    /// and also if the asset is active.
    /// The error thrown allows users to understand why the asset is not valid for trading.
    pub fn validate(&self, time: u32) -> PocketResult<()> {
        if !self.is_active {
            return Err(PocketError::InvalidAsset("Asset is not active".into()));
        }
        if !self.allowed_candles.contains(&CandleLength::from(time))
            && !Self::DEFAULT_CANDLE_LENGTHS.contains(&CandleLength::from(time))
        {
            return Err(PocketError::InvalidAsset(format!(
                "Time is not in allowed candle durations, available {:?}",
                self.allowed_candles()
                    .iter()
                    .map(|c| c.duration())
                    .collect::<Vec<_>>()
            )));
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
        #[allow(unused)]
        struct AssetRaw {
            id: i32,
            symbol: String,
            name: String,
            asset_type: AssetType,
            in1: i32,
            payout: i32,
            in3: i32,
            in4: i32,
            in5: i32,
            otc: i32,
            in7: i32,
            in8: i32,
            arr: Vec<String>,
            in9: i64,
            valid: bool,
            times: Vec<CandleLength>,
            in10: i32,
            in11: i32,
            in12: i64,
        }

        let raw: AssetRaw = AssetRaw::deserialize(deserializer)?;
        Ok(Asset {
            id: raw.id,
            symbol: raw.symbol,
            name: raw.name,
            is_otc: raw.otc == 1,
            is_active: raw.valid,
            payout: raw.payout,
            allowed_candles: raw.times,
            asset_type: raw.asset_type,
        })
    }
}

/// Wrapper around HashMap<String, Asset>
#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Call, // Buy
    Put,  // Sell
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailOpenOrder {
    pub error: String,
    pub amount: f64,
    pub asset: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrder {
    asset: String,
    action: Action,
    amount: f64,
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
    #[serde(with = "float_time")]
    pub open_timestamp: DateTime<Utc>,
    #[serde(with = "float_time")]
    pub close_timestamp: DateTime<Utc>,
    pub refund_time: Option<Value>,
    pub refund_timestamp: Option<Value>,
    pub uid: u64,
    pub request_id: Option<Uuid>,
    pub amount: f64,
    pub profit: f64,
    pub percent_profit: i32,
    pub percent_loss: i32,
    pub open_price: f64,
    pub close_price: f64,
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
    pub amount_usd: Option<f64>,
    #[serde(rename = "amountUSD")]
    pub amount_usd2: Option<f64>,
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
        amount: f64,
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
    pub amount: f64,
    pub symbol: String,
    pub open_time: String,
    pub open_price: f64,
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
    amount: f64,
    asset: String,
    open_time: u32,
    open_price: f64,
    timeframe: u32,
    min_payout: u32,
    command: u32,
}

impl OpenPendingOrder {
    pub fn new(
        open_type: u32,
        amount: f64,
        asset: String,
        open_time: u32,
        open_price: f64,
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_order_format() {
        let order = OpenOrder::new(
            1.0,
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

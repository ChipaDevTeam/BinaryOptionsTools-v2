use core::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use binary_options_tools_core::reimports::Message;
use binary_options_tools_core_pre::traits::Rule;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

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

/// Candle data structure for PocketOption price data
///
/// This represents OHLC (Open, High, Low, Close) price data for a specific time period.
/// Note: PocketOption doesn't provide volume data, so the volume field is always None.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    /// Trading symbol (e.g., "EURUSD_otc")
    pub symbol: String,
    /// Unix timestamp of the candle start time
    pub timestamp: f64,
    /// Opening price
    pub open: f64,
    /// Highest price in the candle period
    pub high: f64,
    /// Lowest price in the candle period
    pub low: f64,
    /// Closing price
    pub close: f64,
    /// Volume is not provided by PocketOption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    /// Whether this candle is closed/finalized
    pub is_closed: bool,
}

impl Candle {
    /// Create a new candle with initial price
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol
    /// * `timestamp` - Unix timestamp for the candle start
    /// * `price` - Initial price (used for open, high, low, close)
    ///
    /// # Returns
    /// New Candle instance with all OHLC values set to the initial price
    pub fn new(symbol: String, timestamp: f64, price: f64) -> Self {
        Self {
            symbol,
            timestamp,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: None, // PocketOption doesn't provide volume
            is_closed: false,
        }
    }

    /// Update the candle with a new price
    ///
    /// This method updates the high, low, and close prices while maintaining
    /// the open price from the initial candle creation.
    ///
    /// # Arguments
    /// * `price` - New price to incorporate into the candle
    pub fn update_price(&mut self, price: f64) {
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
    }

    /// Mark the candle as closed/finalized
    ///
    /// Once a candle is closed, it should not be updated with new prices.
    /// This is typically called when a time-based candle period ends.
    pub fn close_candle(&mut self) {
        self.is_closed = true;
    }

    /// Get the price range (high - low) of the candle
    ///
    /// # Returns
    /// Price range as f64
    pub fn price_range(&self) -> f64 {
        self.high - self.low
    }

    /// Check if the candle is bullish (close > open)
    ///
    /// # Returns
    /// True if the candle closed higher than it opened
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Check if the candle is bearish (close < open)
    ///
    /// # Returns
    /// True if the candle closed lower than it opened
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Check if the candle is a doji (close ≈ open)
    ///
    /// # Returns
    /// True if the candle has very little price movement
    pub fn is_doji(&self) -> bool {
        let body_size = (self.close - self.open).abs();
        let range = self.price_range();

        // Consider it a doji if the body is less than 10% of the range
        if range > 0.0 {
            body_size / range < 0.1
        } else {
            true // No price movement at all
        }
    }

    /// Get the body size of the candle (absolute difference between open and close)
    ///
    /// # Returns
    /// Body size as f64
    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Get the upper shadow length
    ///
    /// # Returns
    /// Upper shadow length as f64
    pub fn upper_shadow(&self) -> f64 {
        self.high - self.open.max(self.close)
    }

    /// Get the lower shadow length
    ///
    /// # Returns
    /// Lower shadow length as f64
    pub fn lower_shadow(&self) -> f64 {
        self.open.min(self.close) - self.low
    }

    /// Convert timestamp to DateTime<Utc>
    ///
    /// # Returns
    /// DateTime<Utc> representation of the candle timestamp
    pub fn datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp as i64, 0).unwrap_or_else(Utc::now)
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
    pattern: String
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
use std::time::Duration;

use chrono::{DateTime, Utc};
use rust_decimal::{
    dec,
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    error::{BinaryOptionsError, BinaryOptionsResult},
    pocketoption::error::{PocketError, PocketResult},
};

/// Candle data structure for PocketOption price data
///
/// This represents OHLC (Open, High, Low, Close) price data for a specific time period.
/// Note: PocketOption doesn't provide volume data, so the volume field is always None.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Candle {
    /// Trading symbol (e.g., "EURUSD_otc")
    pub symbol: String,
    /// Unix timestamp of the candle start time
    pub timestamp: f64,
    /// Opening price
    pub open: Decimal,
    /// Highest price in the candle period
    pub high: Decimal,
    /// Lowest price in the candle period
    pub low: Decimal,
    /// Closing price
    pub close: Decimal,
    /// Volume is not provided by PocketOption
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<Decimal>,
    // /// Whether this candle is closed/finalized
    // pub is_closed: bool,
}

#[derive(Debug, Default, Clone)]
/// Base candle structure matching the server's data format.
///
/// The field order matches the server's JSON array format: `[timestamp, open, close, high, low]`.
///
/// # Example JSON
/// ```json
/// [1754529180, 0.92124, 0.92155, 0.92162, 0.92124]
/// ```
pub struct BaseCandle {
    pub timestamp: f64,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: Option<f64>,
}

impl<'de> Deserialize<'de> for BaseCandle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BaseCandleVisitor;

        impl<'de> serde::de::Visitor<'de> for BaseCandleVisitor {
            type Value = BaseCandle;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of 5 or 6 floats")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let timestamp = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let open = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                let close = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
                let high = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
                let low = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;
                let volume: Option<Option<f64>> = seq.next_element()?;
                let volume = volume.flatten();

                Ok(BaseCandle {
                    timestamp,
                    open,
                    close,
                    high,
                    low,
                    volume,
                })
            }
        }

        deserializer.deserialize_seq(BaseCandleVisitor)
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum HistoryItem {
    Tick([f64; 2]),                                    // [timestamp, price]
    TickWithNull([f64; 2], Option<serde_json::Value>), // [timestamp, price, null]
}

impl HistoryItem {
    pub fn to_tick(&self) -> (f64, f64) {
        match self {
            HistoryItem::Tick([t, p]) => (*t, *p),
            HistoryItem::TickWithNull([t, p], _) => (*t, *p),
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct CandleItem(pub f64, pub f64, pub f64, pub f64, pub f64, pub f64); // timestamp, open, close, high, low, volume

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
    pub fn new(symbol: String, timestamp: f64, price: f64) -> BinaryOptionsResult<Self> {
        let price = Decimal::from_f64(price).ok_or(BinaryOptionsError::General(
            "Couldn't parse f64 to Decimal".to_string(),
        ))?;
        Ok(Self {
            symbol,
            timestamp,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: None, // PocketOption doesn't provide volume
                          // is_closed: false,
        })
    }

    /// Update the candle with a new price
    ///
    /// This method updates the high, low, and close prices while maintaining
    /// the open price from the initial candle creation.
    ///
    /// # Arguments
    /// * `price` - New price to incorporate into the candle
    pub fn update_price(&mut self, price: f64) -> BinaryOptionsResult<()> {
        let price = Decimal::from_f64(price).ok_or(BinaryOptionsError::General(
            "Couldn't parse f64 to Decimal".to_string(),
        ))?;
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
        Ok(())
    }

    /// Update the candle with a new timestamp and price
    ///
    /// This method updates the high, low, and close prices while maintaining
    /// the open price from the initial candle creation.
    ///
    /// # Arguments
    /// * `timestamp` - New timestamp for the candle
    /// * `price` - New price to incorporate into the candle
    pub fn update(&mut self, timestamp: f64, price: f64) -> BinaryOptionsResult<()> {
        let price = Decimal::from_f64(price).ok_or(BinaryOptionsError::General(
            "Couldn't parse f64 to Decimal".to_string(),
        ))?;

        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
        self.timestamp = timestamp;
        Ok(())
    }

    // /// Mark the candle as closed/finalized
    // ///
    // /// Once a candle is closed, it should not be updated with new prices.
    // /// This is typically called when a time-based candle period ends.
    // pub fn close_candle(&mut self) {
    //     self.is_closed = true;
    // }

    /// Get the price range (high - low) of the candle
    ///
    /// # Returns
    /// Price range as Decimal
    pub fn price_range(&self) -> Decimal {
        self.high - self.low
    }

    pub fn price_range_f64(&self) -> BinaryOptionsResult<f64> {
        self.price_range()
            .to_f64()
            .ok_or(BinaryOptionsError::ParseDecimal(
                "Couldn't parse Decimal to f64".to_string(),
            ))
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
        if range > dec!(0.0) {
            body_size / range < dec!(0.1)
        } else {
            true // No price movement at all
        }
    }

    /// Get the body size of the candle (absolute difference between open and close)
    ///
    /// # Returns
    /// Body size as Decimal
    pub fn body_size(&self) -> Decimal {
        (self.close - self.open).abs()
    }

    /// Get the body size of the candle (absolute difference between open and close)
    ///
    /// # Returns
    /// Body size as f64
    pub fn body_size_f64(&self) -> BinaryOptionsResult<f64> {
        self.body_size()
            .to_f64()
            .ok_or(BinaryOptionsError::ParseDecimal(
                "Couldn't parse Decimal to f64".to_string(),
            ))
    }

    /// Get the upper shadow length
    ///
    /// # Returns
    /// Upper shadow length as Decimal
    pub fn upper_shadow(&self) -> Decimal {
        self.high - self.open.max(self.close)
    }

    /// Get the upper shadow length
    ///
    /// # Returns
    /// Upper shadow length as f64
    pub fn upper_shadow_f64(&self) -> BinaryOptionsResult<f64> {
        self.upper_shadow()
            .to_f64()
            .ok_or(BinaryOptionsError::ParseDecimal(
                "Couldn't parse Decimal to f64".to_string(),
            ))
    }

    /// Get the lower shadow length
    ///
    /// # Returns
    /// Lower shadow length as Decimal
    pub fn lower_shadow(&self) -> Decimal {
        self.open.min(self.close) - self.low
    }

    /// Get the lower shadow length
    ///
    /// # Returns
    /// Lower shadow length as f64
    pub fn lower_shadow_f64(&self) -> BinaryOptionsResult<f64> {
        self.lower_shadow()
            .to_f64()
            .ok_or(BinaryOptionsError::ParseDecimal(
                "Couldn't parse Decimal to f64".to_string(),
            ))
    }

    /// Convert timestamp to DateTime<Utc>
    ///
    /// # Returns
    /// DateTime<Utc> representation of the candle timestamp
    pub fn datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp as i64, 0).unwrap_or_else(Utc::now)
    }
}

/// Represents the type of subscription for candle data.
#[derive(Clone, Debug)]
pub enum SubscriptionType {
    None,
    Chunk {
        size: usize,        // Number of candles to aggregate
        current: usize,     // Current aggregated candle count
        candle: BaseCandle, // Current aggregated candle
    },
    Time {
        start_time: Option<f64>,
        duration: Duration,
        candle: BaseCandle,
    },
    TimeAligned {
        duration: Duration,
        candle: BaseCandle,
        /// Stores the timestamp for the end of the current aggregation window.
        next_boundary: Option<f64>,
    },
}

impl BaseCandle {
    pub fn new(
        timestamp: f64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<f64>,
    ) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume, // PocketOption doesn't provide volume
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp as i64, 0).unwrap_or_else(Utc::now)
    }
}

/// Compiles raw tick data into candles based on the specified period.
///
/// # Arguments
/// * `ticks` - Slice of history items (ticks)
/// * `period` - Time period in seconds for each candle. Must be greater than 0.
/// * `symbol` - Trading symbol
///
/// # Returns
/// Vector of compiled Candles. Returns an empty vector if:
/// * `ticks` is empty
/// * `period` is 0 (to avoid division by zero)
pub fn compile_candles_from_ticks(ticks: &[HistoryItem], period: u32, symbol: &str) -> Vec<Candle> {
    if ticks.is_empty() || period == 0 {
        return Vec::new();
    }

    let mut candles = Vec::new();
    let period_secs = period as f64;

    // Sort ticks by timestamp just in case
    let mut sorted_ticks: Vec<(f64, f64)> = ticks.iter().map(|t| t.to_tick()).collect();
    sorted_ticks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut current_candle: Option<BaseCandle> = None;
    let mut current_boundary_idx: Option<u64> = None;

    for (timestamp, price) in sorted_ticks {
        let boundary_idx = (timestamp / period_secs).floor() as u64;
        let boundary = boundary_idx as f64 * period_secs;

        if let Some(mut candle) = current_candle.take() {
            if Some(boundary_idx) == current_boundary_idx {
                // Same candle
                candle.high = candle.high.max(price);
                candle.low = candle.low.min(price);
                candle.close = price;
                current_candle = Some(candle);
            } else {
                // New candle, push old one
                match Candle::try_from((candle, symbol.to_string())) {
                    Ok(c) => candles.push(c),
                    Err(e) => warn!("Failed to convert final candle for {}: {}", symbol, e),
                }
                // Start new candle
                current_boundary_idx = Some(boundary_idx);
                current_candle = Some(BaseCandle {
                    timestamp: boundary,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: None,
                });
            }
        } else {
            // First tick
            current_boundary_idx = Some(boundary_idx);
            current_candle = Some(BaseCandle {
                timestamp: boundary,
                open: price,
                high: price,
                low: price,
                close: price,
                volume: None,
            });
        }
    }

    if let Some(candle) = current_candle {
        match Candle::try_from((candle, symbol.to_string())) {
            Ok(c) => candles.push(c),
            Err(e) => warn!("Failed to convert final candle for {}: {}", symbol, e),
        }
    }

    candles
}

impl SubscriptionType {
    pub fn none() -> Self {
        SubscriptionType::None
    }

    pub fn chunk(size: usize) -> Self {
        SubscriptionType::Chunk {
            size,
            current: 0,
            candle: BaseCandle::default(),
        }
    }

    pub fn time(duration: Duration) -> Self {
        SubscriptionType::Time {
            start_time: None,
            duration,
            candle: BaseCandle::default(),
        }
    }

    /// Creates a time-aligned subscription.
    ///
    /// Completed candle timestamps are set to the boundary start time (the beginning of the aggregation window).
    pub fn time_aligned(duration: Duration) -> PocketResult<Self> {
        if !(24 * 60 * 60 % duration.as_secs() == 0) {
            warn!(
                "Unsupported duration for time-aligned subscription: {:?}",
                duration
            );
            return Err(PocketError::General(format!(
                "Unsupported duration for time-aligned subscription: {duration:?}, duration should be a multiple of the number of seconds in a day"
            )));
        }
        Ok(SubscriptionType::TimeAligned {
            duration,
            candle: BaseCandle::default(),
            next_boundary: None,
        })
    }

    pub fn period_secs(&self) -> Option<u32> {
        match self {
            SubscriptionType::Time { duration, .. } => Some(duration.as_secs() as u32),
            SubscriptionType::TimeAligned { duration, .. } => Some(duration.as_secs() as u32),
            _ => None,
        }
    }

    pub fn update(&mut self, new_candle: &BaseCandle) -> PocketResult<Option<BaseCandle>> {
        match self {
            SubscriptionType::None => Ok(Some(new_candle.clone())),

            SubscriptionType::Chunk {
                size,
                current,
                candle,
            } => {
                if *current == 0 {
                    *candle = new_candle.clone();
                } else {
                    candle.timestamp = new_candle.timestamp;
                    candle.high = candle.high.max(new_candle.high);
                    candle.low = candle.low.min(new_candle.low);
                    candle.close = new_candle.close;
                }
                *current += 1;

                if *current >= *size {
                    *current = 0; // Reset for next batch
                    Ok(Some(candle.clone()))
                } else {
                    Ok(None)
                }
            }

            SubscriptionType::Time {
                start_time,
                duration,
                candle,
            } => {
                if start_time.is_none() {
                    *start_time = Some(new_candle.timestamp);
                    *candle = new_candle.clone();
                    return Ok(None);
                }

                // Update the aggregated candle
                candle.timestamp = new_candle.timestamp;
                candle.high = candle.high.max(new_candle.high);
                candle.low = candle.low.min(new_candle.low);
                candle.close = new_candle.close;

                let elapsed = (new_candle.timestamp()
                    - DateTime::from_timestamp(start_time.unwrap() as i64, 0)
                        .unwrap_or_else(Utc::now))
                .to_std()
                .map_err(|_| {
                    PocketError::General("Time calculation error in conditional update".to_string())
                })?;

                if elapsed >= *duration {
                    *start_time = None; // Reset for next period
                    Ok(Some(candle.clone()))
                } else {
                    Ok(None)
                }
            }

            SubscriptionType::TimeAligned {
                duration,
                candle,
                next_boundary,
            } => {
                let boundary = match *next_boundary {
                    Some(b) => b,
                    None => {
                        // First candle ever processed. Initialize the state.
                        *candle = new_candle.clone();
                        let duration_secs = duration.as_secs_f64();
                        let bucket_id = (new_candle.timestamp / duration_secs).floor();
                        let new_boundary = (bucket_id + 1.0) * duration_secs;
                        *next_boundary = Some(new_boundary);

                        // It's the first candle, so the window can't be complete yet.
                        return Ok(None);
                    }
                };

                if new_candle.timestamp < boundary {
                    // The new candle is within the current time window. Aggregate its data.
                    candle.high = candle.high.max(new_candle.high);
                    candle.low = candle.low.min(new_candle.low);
                    candle.close = new_candle.close;
                    candle.timestamp = new_candle.timestamp;
                    if let (Some(v_agg), Some(v_new)) = (&mut candle.volume, new_candle.volume) {
                        *v_agg += v_new;
                    } else if new_candle.volume.is_some() {
                        candle.volume = new_candle.volume;
                    }
                    Ok(None) // The candle is not yet complete.
                } else {
                    // The new candle's timestamp is at or after the boundary.
                    // The current aggregation window is now complete.
                    // Set timestamp to the start of the period (boundary - duration)
                    candle.timestamp = boundary - duration.as_secs_f64();
                    // 1. Clone the completed candle to return it later.
                    let completed_candle = candle.clone();

                    // 2. Start the new aggregation period with the new_candle's data.
                    *candle = new_candle.clone();

                    // 3. Calculate the boundary for this new period.
                    let duration_secs = duration.as_secs_f64();
                    let bucket_id = (new_candle.timestamp / duration_secs).floor();
                    let new_boundary = (bucket_id + 1.0) * duration_secs;
                    *next_boundary = Some(new_boundary);

                    // 4. Return the candle that was just completed.
                    Ok(Some(completed_candle))
                }
            }
        }
    }
}

impl From<(f64, f64)> for BaseCandle {
    fn from((timestamp, price): (f64, f64)) -> Self {
        BaseCandle {
            timestamp,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: None, // PocketOption doesn't provide volume
        }
    }
}

impl TryFrom<(BaseCandle, String)> for Candle {
    type Error = BinaryOptionsError;

    fn try_from(value: (BaseCandle, String)) -> Result<Self, Self::Error> {
        let (base_candle, symbol) = value;
        let volume = match base_candle.volume {
            Some(v) => Some(
                Decimal::from_f64(v)
                    .ok_or(BinaryOptionsError::General("Couldn't parse volume".into()))?,
            ),
            None => None,
        };
        Ok(Candle {
            symbol,
            timestamp: base_candle.timestamp,
            open: Decimal::from_f64(base_candle.open)
                .ok_or(BinaryOptionsError::General("Couldn't parse open".into()))?,
            high: Decimal::from_f64(base_candle.high)
                .ok_or(BinaryOptionsError::General("Couldn't parse high".into()))?,
            low: Decimal::from_f64(base_candle.low)
                .ok_or(BinaryOptionsError::General("Couldn't parse low".into()))?,
            close: Decimal::from_f64(base_candle.close)
                .ok_or(BinaryOptionsError::General("Couldn't parse close".into()))?,
            volume,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base_candles() {
        // Format: [timestamp, open, close, high, low]
        let data = r#"[1754529180,0.92124,0.92155,0.92162,0.92124]"#;
        let candle: BaseCandle = serde_json::from_str(data).unwrap();
        assert_eq!(candle.timestamp, 1754529180.0);
        assert_eq!(candle.open, 0.92124);
        assert_eq!(candle.close, 0.92155);
        assert_eq!(candle.high, 0.92162);
        assert_eq!(candle.low, 0.92124);
        assert_eq!(candle.volume, None);
    }

    #[test]
    fn test_parse_base_candles_with_volume() {
        // Format: [timestamp, open, close, high, low, volume]
        let data = r#"[1754529180,0.92124,0.92155,0.92162,0.92124,100.0]"#;
        let candle: BaseCandle = serde_json::from_str(data).unwrap();
        assert_eq!(candle.volume, Some(100.0));
    }

    #[test]
    fn test_parse_base_candles_with_null_volume() {
        // Format: [timestamp, open, close, high, low, null]
        let data = r#"[1754529180,0.92124,0.92155,0.92162,0.92124,null]"#;
        let candle: BaseCandle = serde_json::from_str(data).unwrap();
        assert_eq!(candle.volume, None);
    }

    #[test]
    fn test_compile_candles_zero_period() {
        let ticks = vec![
            HistoryItem::Tick([1000.0, 1.0]),
            HistoryItem::Tick([1001.0, 1.1]),
        ];
        let candles = compile_candles_from_ticks(&ticks, 0, "TEST");
        assert!(candles.is_empty());
    }

    #[test]
    fn test_compile_candles_empty_ticks() {
        let ticks = vec![];
        let candles = compile_candles_from_ticks(&ticks, 60, "TEST");
        assert!(candles.is_empty());
    }

    #[test]
    fn test_compile_candles_single_tick() {
        let ticks = vec![HistoryItem::Tick([1000.0, 1.5])];
        let candles = compile_candles_from_ticks(&ticks, 60, "TEST");
        assert_eq!(candles.len(), 1);
        let c = &candles[0];
        // 1000 / 60 = 16.66.. -> floor 16. 16 * 60 = 960.
        // So timestamp should be 960.
        assert_eq!(c.timestamp, 960.0);
        assert_eq!(c.open.to_string(), "1.5");
        assert_eq!(c.high.to_string(), "1.5");
        assert_eq!(c.low.to_string(), "1.5");
        assert_eq!(c.close.to_string(), "1.5");
    }
}

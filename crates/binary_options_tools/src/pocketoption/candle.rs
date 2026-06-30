#![allow(clippy::items_after_test_module)]

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
    pocketoption::utils::normalize_timestamp,
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
    pub timestamp: i64,
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
    /// Whether this candle is closed/finalized
    pub is_closed: bool,
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
    pub timestamp: i64,
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
                formatter.write_str("a sequence of 5 or 6 elements")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let timestamp_raw: f64 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                let timestamp = normalize_timestamp(timestamp_raw);
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
    Tick([serde_json::Value; 2]),
    TickWithNull([serde_json::Value; 3]),
    Candle(CandleItem),
}

impl HistoryItem {
    pub fn to_tick(&self) -> (i64, f64) {
        match self {
            HistoryItem::Tick([t, p]) => {
                let ts = t.as_f64().unwrap_or_default();
                let timestamp = normalize_timestamp(ts);
                (timestamp, p.as_f64().unwrap_or_default())
            }
            HistoryItem::TickWithNull([t, p, _]) => {
                let ts = t.as_f64().unwrap_or_default();
                let timestamp = normalize_timestamp(ts);
                (timestamp, p.as_f64().unwrap_or_default())
            }
            HistoryItem::Candle(c) => (c.timestamp, c.close),
        }
    }
}

/// Raw candle item from server responses: [timestamp, open, close, high, low, volume]
/// Timestamp is automatically normalized from milliseconds if needed.
#[derive(Debug, Clone)]
pub struct CandleItem {
    pub timestamp: i64,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
}

impl<'de> Deserialize<'de> for CandleItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CandleItemVisitor;

        impl<'de> serde::de::Visitor<'de> for CandleItemVisitor {
            type Value = CandleItem;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "a sequence of 6 elements: [timestamp, open, close, high, low, volume]",
                )
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let timestamp_raw: f64 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let timestamp = normalize_timestamp(timestamp_raw);
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
                let volume = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;

                Ok(CandleItem {
                    timestamp,
                    open,
                    close,
                    high,
                    low,
                    volume,
                })
            }
        }

        deserializer.deserialize_seq(CandleItemVisitor)
    }
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
    pub fn new(symbol: String, timestamp: i64, price: f64) -> BinaryOptionsResult<Self> {
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
            is_closed: false,
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
    pub fn update(&mut self, timestamp: i64, price: f64) -> BinaryOptionsResult<()> {
        let price = Decimal::from_f64(price).ok_or(BinaryOptionsError::General(
            "Couldn't parse f64 to Decimal".to_string(),
        ))?;

        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
        self.timestamp = timestamp;
        Ok(())
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

    /// Convert timestamp to `DateTime<Utc>`
    ///
    /// # Returns
    /// `DateTime<Utc>` representation of the candle timestamp
    pub fn datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp, 0).unwrap_or_else(Utc::now)
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
        start_time: Option<i64>,
        duration: Duration,
        candle: BaseCandle,
    },
    TimeAligned {
        duration: Duration,
        candle: BaseCandle,
        /// Stores the timestamp for the end of the current aggregation window.
        next_boundary: Option<i64>,
    },
}

impl BaseCandle {
    pub fn new(
        timestamp: i64,
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
        DateTime::from_timestamp(self.timestamp, 0).unwrap_or_else(Utc::now)
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
    let period_i64 = period as i64;

    // Sort ticks by timestamp just in case
    let mut sorted_ticks: Vec<(i64, f64)> = ticks.iter().map(|t| t.to_tick()).collect();
    sorted_ticks.sort_by(|a, b| a.0.cmp(&b.0));

    let mut current_candle: Option<BaseCandle> = None;
    let mut current_boundary_idx: Option<i64> = None;

    for (timestamp, price) in sorted_ticks {
        // Timestamps are already normalized to seconds by to_tick()
        let boundary_idx = timestamp / period_i64;
        let boundary = boundary_idx * period_i64;

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
        if 24 * 60 * 60 % duration.as_secs() != 0 {
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
                    // First candle in chunk - preserve its timestamp as the chunk start time
                    *candle = new_candle.clone();
                } else {
                    // Keep the original chunk start timestamp - DO NOT update with latest candle time
                    // This ensures aggregated candles are properly aligned to their start boundary
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

                // Update the aggregated candle - preserve the start timestamp (industry standard for OHLC)
                candle.high = candle.high.max(new_candle.high);
                candle.low = candle.low.min(new_candle.low);
                candle.close = new_candle.close;

                let elapsed = (new_candle.timestamp()
                    - DateTime::from_timestamp(start_time.unwrap(), 0).unwrap_or_else(Utc::now))
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
                        let duration_secs = duration.as_secs() as i64;
                        let bucket_id = new_candle.timestamp / duration_secs;
                        let new_boundary = (bucket_id + 1) * duration_secs;
                        *next_boundary = Some(new_boundary);

                        // It's the first candle, so the window can't be complete yet.
                        return Ok(None);
                    }
                };

                if new_candle.timestamp < boundary {
                    // The new candle is within the current time window. Aggregate its data.
                    // Do NOT update the timestamp - preserve the start of the aggregation window.
                    candle.high = candle.high.max(new_candle.high);
                    candle.low = candle.low.min(new_candle.low);
                    candle.close = new_candle.close;
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
                    let duration_secs = duration.as_secs() as i64;
                    candle.timestamp = boundary - duration_secs;
                    // 1. Clone the completed candle to return it later.
                    let completed_candle = candle.clone();

                    // 2. Start the new aggregation period with the new_candle's data.
                    *candle = new_candle.clone();

                    // 3. Calculate the boundary for this new period.
                    let bucket_id = new_candle.timestamp / duration_secs;
                    let new_boundary = (bucket_id + 1) * duration_secs;
                    *next_boundary = Some(new_boundary);

                    // 4. Return the candle that was just completed.
                    Ok(Some(completed_candle))
                }
            }
        }
    }
}

impl From<(i64, f64)> for BaseCandle {
    fn from((timestamp, price): (i64, f64)) -> Self {
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
            is_closed: false,
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
        assert_eq!(candle.timestamp, 1754529180);
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
            HistoryItem::Tick([1000.into(), 1.0.into()]),
            HistoryItem::Tick([1001.into(), 1.1.into()]),
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
        let ticks = vec![HistoryItem::Tick([1000.into(), 1.5.into()])];
        let candles = compile_candles_from_ticks(&ticks, 60, "TEST");
        assert_eq!(candles.len(), 1);
        let c = &candles[0];
        // 1000 / 60 = 16.66.. -> floor 16. 16 * 60 = 960.
        // So timestamp should be 960.
        assert_eq!(c.timestamp, 960);
        assert_eq!(c.open.to_string(), "1.5");
        assert_eq!(c.high.to_string(), "1.5");
        assert_eq!(c.low.to_string(), "1.5");
        assert_eq!(c.close.to_string(), "1.5");
    }

    #[test]
    fn test_compile_candles_millisecond_timestamps() {
        // Timestamps in milliseconds (1.7e12 is year 2024)
        let ticks = vec![
            HistoryItem::Tick([1714529180000u64.into(), 1.0.into()]),
            HistoryItem::Tick([1714529181000u64.into(), 1.1.into()]),
            HistoryItem::Tick([1714529240000u64.into(), 1.2.into()]),
        ];
        let candles = compile_candles_from_ticks(&ticks, 60, "TEST");

        assert_eq!(candles.len(), 2);
        // Should be normalized to seconds boundaries
        assert_eq!(candles[0].timestamp, 1714529160);
        assert_eq!(candles[1].timestamp, 1714529220);
    }

    #[test]
    fn test_compile_candles_from_tuples_simple() {
        let ticks = vec![
            (1000, 1.5),
            (1001, 1.6),
            (1002, 1.7),
            (1003, 1.8),
            (1004, 1.9),
            (1005, 2.0),
        ];
        let candles = compile_candles_from_tuples(&ticks, 3, "TEST");
        // With period=3, timestamps: 1000,1001,1002 -> boundaries: 999,1002,1005
        assert_eq!(candles.len(), 3);
        assert_eq!(candles[0].timestamp, 999);
        assert_eq!(candles[0].open.to_string(), "1.5");
        assert_eq!(candles[0].high.to_string(), "1.6");
        assert_eq!(candles[0].low.to_string(), "1.5");
        assert_eq!(candles[0].close.to_string(), "1.6");
        assert_eq!(candles[1].timestamp, 1002);
        assert_eq!(candles[1].open.to_string(), "1.7");
        assert_eq!(candles[1].high.to_string(), "1.9");
        assert_eq!(candles[1].low.to_string(), "1.7");
        assert_eq!(candles[1].close.to_string(), "1.9");
        assert_eq!(candles[2].timestamp, 1005);
        assert_eq!(candles[2].open.to_string(), "2");
        assert_eq!(candles[2].high.to_string(), "2");
        assert_eq!(candles[2].low.to_string(), "2");
        assert_eq!(candles[2].close.to_string(), "2");
    }

    #[test]
    fn test_compile_candles_from_tuples_empty() {
        let ticks = vec![];
        let candles = compile_candles_from_tuples(&ticks, 60, "TEST");
        assert!(candles.is_empty());
    }

    #[test]
    fn test_compile_candles_from_tuples_zero_period() {
        let ticks = vec![(1000, 1.5), (1001, 1.6)];
        let candles = compile_candles_from_tuples(&ticks, 0, "TEST");
        assert!(candles.is_empty());
    }

    #[test]
    fn test_compile_candles_from_tuples_unaligned() {
        // Test with timestamps that don't align to period boundaries
        let ticks = vec![
            (1001, 1.5), // 1001/20 = 50, boundary = 1000
            (1015, 1.6), // 1015/20 = 50, boundary = 1000
            (1020, 1.7), // 1020/20 = 51, boundary = 1020
            (1035, 1.8), // 1035/20 = 51, boundary = 1020
            (1040, 1.9), // 1040/20 = 52, boundary = 1040
        ];
        let candles = compile_candles_from_tuples(&ticks, 20, "TEST");
        assert_eq!(candles.len(), 3);
        assert_eq!(candles[0].timestamp, 1000);
        assert_eq!(candles[1].timestamp, 1020);
        assert_eq!(candles[2].timestamp, 1040);
    }

    #[test]
    fn test_normalize_timestamp_seconds_truncation() {
        use crate::pocketoption::utils::normalize_timestamp;
        // Sub-second timestamps should be truncated, not rounded
        assert_eq!(normalize_timestamp(1774789371.94), 1774789371);
        assert_eq!(normalize_timestamp(1774789371.50), 1774789371);
        assert_eq!(normalize_timestamp(1774789371.49), 1774789371);
        assert_eq!(normalize_timestamp(1774789371.00), 1774789371);
    }

    #[test]
    fn test_normalize_timestamp_milliseconds() {
        use crate::pocketoption::utils::normalize_timestamp;
        // Millisecond timestamps should be divided by 1000 and truncated
        assert_eq!(normalize_timestamp(1714529180000.0), 1714529180);
        assert_eq!(normalize_timestamp(1714529180500.0), 1714529180);
        assert_eq!(normalize_timestamp(1714529180900.0), 1714529180);
    }

    #[test]
    fn test_base_candle_ms_timestamp_truncation() {
        // BaseCandle deserializer should truncate (not round) ms timestamps
        let data = r#"[1714529180500.0,0.92124,0.92155,0.92162,0.92124]"#;
        let candle: BaseCandle = serde_json::from_str(data).unwrap();
        // 1714529180500 / 1000 = 1714529180.5 -> truncates to 1714529180
        assert_eq!(candle.timestamp, 1714529180);
    }

    #[test]
    fn test_base_candle_second_timestamp_truncation() {
        // BaseCandle deserializer should truncate sub-second timestamps
        let data = r#"[1774789371.94,0.92124,0.92155,0.92162,0.92124]"#;
        let candle: BaseCandle = serde_json::from_str(data).unwrap();
        // 1774789371.94 -> truncates to 1774789371
        assert_eq!(candle.timestamp, 1774789371);
    }

    #[test]
    fn test_history_item_ms_timestamp_truncation() {
        // HistoryItem::to_tick() should truncate ms timestamps
        let item = HistoryItem::Tick([serde_json::json!(1714529180500.0), serde_json::json!(1.5)]);
        let (ts, _price) = item.to_tick();
        assert_eq!(ts, 1714529180);
    }

    #[test]
    fn test_history_item_second_timestamp_truncation() {
        // HistoryItem::to_tick() should truncate sub-second timestamps
        let item = HistoryItem::Tick([serde_json::json!(1774789371.94), serde_json::json!(1.5)]);
        let (ts, _price) = item.to_tick();
        assert_eq!(ts, 1774789371);
    }

    #[test]
    fn test_candle_item_ms_timestamp_normalization() {
        // CandleItem deserializer should normalize ms timestamps
        let data = r#"[1714529180500.0,0.92124,0.92155,0.92162,0.92124,100.0]"#;
        let item: CandleItem = serde_json::from_str(data).unwrap();
        assert_eq!(item.timestamp, 1714529180);
    }

    #[test]
    fn test_candle_item_second_timestamp_normalization() {
        // CandleItem deserializer should truncate sub-second timestamps
        let data = r#"[1774789371.94,0.92124,0.92155,0.92162,0.92124,100.0]"#;
        let item: CandleItem = serde_json::from_str(data).unwrap();
        assert_eq!(item.timestamp, 1774789371);
    }

    #[test]
    fn test_subscription_time_preserves_start_timestamp() {
        use std::time::Duration as StdDuration;
        let mut sub = SubscriptionType::time(StdDuration::from_secs(60));

        // First tick at t=1000 - initializes the window
        let tick1 = BaseCandle::new(1000, 1.0, 1.0, 1.0, 1.0, None);
        assert!(sub.update(&tick1).unwrap().is_none());

        // Second tick at t=1030 (within 60s window, updates OHLC but NOT timestamp)
        let tick2 = BaseCandle::new(1030, 1.1, 1.1, 1.1, 1.1, None);
        assert!(sub.update(&tick2).unwrap().is_none());

        // Third tick at t=1060 (exceeds 60s window, should complete candle)
        let tick3 = BaseCandle::new(1060, 1.2, 1.2, 1.2, 1.2, None);
        let completed = sub.update(&tick3).unwrap();
        assert!(completed.is_some());
        let candle = completed.unwrap();
        // Timestamp should be the START of the aggregation window (1000), not the latest tick (1060)
        assert_eq!(candle.timestamp, 1000);
        // Close price includes tick3's data (the candle is completed when tick3 triggers the boundary)
        assert_eq!(candle.close, 1.2);
    }

    #[test]
    fn test_subscription_time_aligned_preserves_start_timestamp() {
        use std::time::Duration as StdDuration;
        let mut sub = SubscriptionType::time_aligned(StdDuration::from_secs(60)).unwrap();

        // First tick at t=1000 - initializes window
        // bucket_id = 1000/60 = 16, boundary = 17*60 = 1020
        let tick1 = BaseCandle::new(1000, 1.0, 1.0, 1.0, 1.0, None);
        assert!(sub.update(&tick1).unwrap().is_none());

        // Second tick at t=1010 (within same 60s window, 1010 < 1020)
        let tick2 = BaseCandle::new(1010, 1.1, 1.1, 1.1, 1.1, None);
        assert!(sub.update(&tick2).unwrap().is_none());

        // Third tick at t=1020 (at boundary, should complete first candle)
        let tick3 = BaseCandle::new(1020, 1.2, 1.2, 1.2, 1.2, None);
        let completed = sub.update(&tick3).unwrap();
        assert!(completed.is_some());
        let candle = completed.unwrap();
        // Timestamp should be the START of the completed period
        // boundary was 1020, duration=60, so start = 1020 - 60 = 960
        assert_eq!(candle.timestamp, 960);
    }

    #[test]
    fn test_get_index_uniqueness() {
        use crate::pocketoption::utils::get_index;
        // Generate 1000 indices and verify all are unique
        let indices: Vec<u64> = (0..1000).map(|_| get_index().unwrap()).collect();
        let unique: std::collections::HashSet<u64> = indices.iter().copied().collect();
        assert_eq!(indices.len(), unique.len(), "All indices should be unique");
    }

    #[test]
    fn test_cross_path_timestamp_consistency() {
        use crate::pocketoption::utils::normalize_timestamp;
        // Verify that all code paths produce the same result for the same input
        let raw_seconds = 1774789371.94;
        let raw_ms = 1774789371940.0;

        // Both should normalize to the same second
        let from_seconds = normalize_timestamp(raw_seconds);
        let from_ms = normalize_timestamp(raw_ms);
        assert_eq!(
            from_seconds, from_ms,
            "Second and ms paths should produce same result"
        );

        // BaseCandle deserializer should match
        let data_sec = format!(r#"[{raw_seconds},0.92124,0.92155,0.92162,0.92124]"#);
        let candle: BaseCandle = serde_json::from_str(&data_sec).unwrap();
        assert_eq!(
            candle.timestamp, from_seconds,
            "BaseCandle should match normalize_timestamp"
        );

        // HistoryItem should match
        let item = HistoryItem::Tick([serde_json::json!(raw_seconds), serde_json::json!(1.5)]);
        let (ts, _) = item.to_tick();
        assert_eq!(
            ts, from_seconds,
            "HistoryItem should match normalize_timestamp"
        );

        // CandleItem should match
        let ci_data = format!(r#"[{raw_seconds},0.92124,0.92155,0.92162,0.92124,100.0]"#);
        let ci: CandleItem = serde_json::from_str(&ci_data).unwrap();
        assert_eq!(
            ci.timestamp, from_seconds,
            "CandleItem should match normalize_timestamp"
        );
    }

    #[test]
    fn test_history_item_candle() {
        let candle_item = CandleItem {
            timestamp: 1000,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 100.0,
        };
        let item = HistoryItem::Candle(candle_item);
        let (ts, price) = item.to_tick();
        assert_eq!(ts, 1000);
        assert_eq!(price, 1.5);
    }
}

/// Compiles raw tick data (timestamp, price tuples) into custom-period candles.
///
/// This is a convenience function that works with the output of `ticks()`.
///
/// # Arguments
/// * `ticks` - Slice of (timestamp, price) tuples
/// * `period` - Time period in seconds for each candle. Must be greater than 0.
/// * `symbol` - Trading symbol
///
/// # Returns
/// Vector of compiled Candles. Returns an empty vector if:
/// * `ticks` is empty
/// * `period` is 0 (to avoid division by zero)
pub fn compile_candles_from_tuples(ticks: &[(i64, f64)], period: u32, symbol: &str) -> Vec<Candle> {
    if ticks.is_empty() || period == 0 {
        return Vec::new();
    }

    // Convert tuples to HistoryItem::Tick format
    let history_items: Vec<HistoryItem> = ticks
        .iter()
        .map(|&(timestamp, price)| HistoryItem::Tick([timestamp.into(), price.into()]))
        .collect();

    compile_candles_from_ticks(&history_items, period, symbol)
}

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::pocketoption::error::{PocketError, PocketResult};

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
    pub open: f64,
    /// Highest price in the candle period
    pub high: f64,
    /// Lowest price in the candle period
    pub low: f64,
    /// Closing price
    pub close: f64,
    /// Volume is not provided by PocketOption
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    // /// Whether this candle is closed/finalized
    // pub is_closed: bool,
}

#[derive(Default, Clone, Deserialize)]
pub struct BaseCandle {
    pub timestamp: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub volume: Option<f64>,
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
                          // is_closed: false,
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

    /// Update the candle with a new timestamp and price
    ///
    /// This method updates the high, low, and close prices while maintaining
    /// the open price from the initial candle creation.
    ///
    /// # Arguments
    /// * `timestamp` - New timestamp for the candle
    /// * `price` - New price to incorporate into the candle
    pub fn update(&mut self, timestamp: f64, price: f64) {
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
        self.timestamp = timestamp;
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

/// Represents the type of subscription for candle data.
#[derive(Clone)]
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

impl SubscriptionType {
    const SUPPORTED_DURATIONS: &[u64] = &[
        5, 15, 30, 60, 120, 180, 300, 600, 900, 1800, 2700, 3600, 7200, 10800, 14400,
    ];

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

    pub fn time_aligned(duration: Duration) -> PocketResult<Self> {
        if !Self::SUPPORTED_DURATIONS.contains(&duration.as_secs()) {
            warn!(
                "Unsupported duration for time-aligned subscription: {:?}",
                duration
            );
            return Err(PocketError::General(format!(
                "Unsupported duration for time-aligned subscription: {duration:?}"
            )));
        }
        Ok(SubscriptionType::TimeAligned {
            duration,
            candle: BaseCandle::default(),
            next_boundary: None,
        })
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
                    candle.timestamp = boundary;
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

impl From<(BaseCandle, String)> for Candle {
    fn from((base_candle, symbol): (BaseCandle, String)) -> Self {
        Candle {
            symbol,
            timestamp: base_candle.timestamp,
            open: base_candle.open,
            high: base_candle.high,
            low: base_candle.low,
            close: base_candle.close,
            volume: base_candle.volume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base_candles() {
        let data = r#"[1754529180,0.92124,0.92155,0.92162,0.92124]"#;
        let candle: BaseCandle = serde_json::from_str(data).unwrap();
        assert_eq!(candle.timestamp, 1754529180.0);
    }
}
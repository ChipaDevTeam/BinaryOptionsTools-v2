use core::fmt;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::pocketoption::error::PocketOptionError;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateStream(pub Vec<UpdateStreamItem>);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateStreamItem {
    pub active: String,
    #[serde(with = "float_time")]
    pub time: DateTime<Utc>,
    pub price: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateHistoryNewFast {
    pub asset: String,
    pub period: i64,
    pub history: Vec<Candle>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoadHistoryPeriodResult {
    pub asset: Option<String>,
    pub index: u64,
    pub data: Vec<Candle>,
    pub period: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Candle {
    Raw(RawCandle),
    Processed(ProcessedCandle),
    Update(UpdateCandle),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RawCandle {
    asset: String,
    #[serde(with = "float_time")]
    time: DateTime<Utc>,
    price: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProcessedCandle {
    symbol_id: i32,
    #[serde(with = "float_time")]
    time: DateTime<Utc>,
    open: f64,
    close: f64,
    high: f64,
    low: f64,
    asset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataCandle {
    pub time: DateTime<Utc>,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateCandle {
    #[serde(with = "float_time")]
    time: DateTime<Utc>,
    price: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBalance {
    is_demo: u32,
    pub balance: f64,
    uid: Option<i64>,
    login: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateAssets(pub Vec<Asset>);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Asset {
    pub id: i32,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub in1: i32,
    pub payout: i32,
    pub in3: i32,
    pub in4: i32,
    pub in5: i32,
    pub in6: i32,
    pub in7: i32,
    pub in8: i32,
    pub arr: Vec<String>,
    pub in9: i64,
    pub val: bool,
    pub times: Vec<TimeCandle>,
    pub in10: i32,
    pub in11: i32,
    pub in12: i64,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeCandle {
    #[serde(with = "duration")]
    time: Duration,
}

impl DataCandle {
    fn new(time: DateTime<Utc>, open: f64, close: f64, high: f64, low: f64) -> Self {
        Self {
            time,
            open,
            close,
            high,
            low,
        }
    }

    fn new_price(time: DateTime<Utc>, price: f64) -> Self {
        Self {
            time,
            open: price,
            close: price,
            high: price,
            low: price,
        }
    }
}

impl From<&Candle> for DataCandle {
    fn from(value: &Candle) -> Self {
        match value {
            Candle::Raw(candle) => Self::new_price(candle.time, candle.price),
            Candle::Processed(candle) => Self::new(
                candle.time,
                candle.open,
                candle.close,
                candle.high,
                candle.low,
            ),
            Candle::Update(candle) => Self::new_price(candle.time, candle.price),
        }
    }
}

impl From<&UpdateStreamItem> for DataCandle {
    fn from(value: &UpdateStreamItem) -> Self {
        Self::new_price(value.time, value.price)
    }
}

impl fmt::Display for DataCandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = serde_json::to_string(&self).map_err(|_| fmt::Error)?;
        raw.fmt(f)
    }
}

impl LoadHistoryPeriodResult {
    pub fn candle_data(&self) -> Vec<DataCandle> {
        self.data.iter().map(DataCandle::from).collect()
    }
    
    /// Returns candle data, aggregating raw ticks if needed based on the requested period
    pub fn candle_data_with_period(&self, requested_period_seconds: i64) -> Vec<DataCandle> {
        // If period is 0 and we have raw data, we need to aggregate
        if self.period == 0 && self.needs_aggregation() {
            self.aggregate_raw_ticks(requested_period_seconds)
        } else {
            // Use existing data as-is
            self.candle_data()
        }
    }
    
    /// Check if the data needs aggregation (contains raw ticks instead of processed candles)
    fn needs_aggregation(&self) -> bool {
        self.data.iter().any(|candle| matches!(candle, Candle::Raw(_)))
    }
    
    /// Aggregate raw tick data into candles for the specified period
    fn aggregate_raw_ticks(&self, period_seconds: i64) -> Vec<DataCandle> {
        if self.data.is_empty() {
            return Vec::new();
        }
        
        let mut result = Vec::new();
        let mut current_bucket: Option<(DateTime<Utc>, Vec<f64>)> = None;
        
        for candle in &self.data {
            if let Candle::Raw(raw_candle) = candle {
                let bucket_start = get_bucket_start(raw_candle.time, period_seconds);
                
                match &mut current_bucket {
                    Some((start_time, prices)) if *start_time == bucket_start => {
                        // Add to current bucket
                        prices.push(raw_candle.price);
                    }
                    _ => {
                        // Finalize previous bucket if exists
                        if let Some((start_time, prices)) = current_bucket.take() {
                            if let Some(aggregated) = create_aggregated_candle(start_time, prices, period_seconds) {
                                result.push(aggregated);
                            }
                        }
                        
                        // Start new bucket
                        current_bucket = Some((bucket_start, vec![raw_candle.price]));
                    }
                }
            }
        }
        
        // Finalize last bucket
        if let Some((start_time, prices)) = current_bucket {
            if let Some(aggregated) = create_aggregated_candle(start_time, prices, period_seconds) {
                result.push(aggregated);
            }
        }
        
        result
    }
}

impl UpdateHistoryNewFast {
    pub fn candle_data(&self) -> Vec<DataCandle> {
        self.history.iter().map(DataCandle::from).collect()
    }
}

/// Get the start time of the bucket for a given timestamp and period
fn get_bucket_start(timestamp: DateTime<Utc>, period_seconds: i64) -> DateTime<Utc> {
    let timestamp_secs = timestamp.timestamp();
    let bucket_start_secs = (timestamp_secs / period_seconds) * period_seconds;
    DateTime::from_timestamp(bucket_start_secs, 0).unwrap_or(timestamp)
}

/// Create an aggregated candle from a list of prices for a time bucket
fn create_aggregated_candle(start_time: DateTime<Utc>, prices: Vec<f64>, _period_seconds: i64) -> Option<DataCandle> {
    if prices.is_empty() {
        return None;
    }
    
    let open = *prices.first()?;
    let close = *prices.last()?;
    let high = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let low = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    
    Some(DataCandle::new(start_time, open, close, high, low))
}

impl Default for UpdateBalance {
    fn default() -> Self {
        Self {
            is_demo: 1,
            balance: -1.,
            uid: None,
            login: None,
        }
    }
}

impl TryFrom<Vec<DataCandle>> for DataCandle {
    type Error = PocketOptionError;

    fn try_from(value: Vec<DataCandle>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(PocketOptionError::EmptyArrayError("DataCandle".into()));
        }
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        let open = value
            .first()
            .ok_or(PocketOptionError::EmptyArrayError("DataCandle".into()))?
            .open;
        let last = value
            .last()
            .ok_or(PocketOptionError::EmptyArrayError("DataCandle".into()))?;
        let close = last.close;
        let time = last.time;
        value.iter().for_each(|c| {
            high = high.max(c.high);
            low = low.min(c.low);
        });
        Ok(DataCandle::new(time, open, close, high, low))
    }
}

pub mod float_time {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.timestamp_millis() as f64 / 1000.0;
        serializer.serialize_f64(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = f64::deserialize(deserializer)?.to_string();
        let (secs, milis) = match s.split_once(".") {
            Some((seconds, miliseconds)) => {
                let secs: i64 = seconds
                    .parse::<i64>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                let mut pow = 0;
                if miliseconds.len() <= 9 {
                    pow = 9u32.saturating_sub(miliseconds.len() as u32);
                }
                let milis = miliseconds
                    .parse::<u32>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?
                    * 10i32.pow(pow) as u32;
                (secs, milis)
            }
            None => {
                let secs: i64 = s
                    .parse::<i64>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;

                (secs, 0)
            }
        };
        DateTime::from_timestamp(secs, milis)
            .ok_or(serde::de::Error::custom("Error parsing ints to time"))
    }
}

pub mod string_time {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer, de};
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let date_str = date.format("%Y-%m-%d %H:%M:%S").to_string();
        serializer.serialize_str(&date_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date_str = String::deserialize(deserializer)?;
        let date = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
            .map_err(de::Error::custom)?
            .and_utc();
        Ok(date)
    }
}

pub mod duration {
    use chrono::Duration;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(duration.num_seconds())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = i64::deserialize(deserializer)?;
        Ok(Duration::seconds(s))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    use std::{error::Error, fs::File, io::BufReader};

    #[test]
    fn test_deserialize_update_stream() -> Result<(), Box<dyn Error>> {
        let tests = [
            r#"[["AUS200_otc",1732830010,6436.06]]"#,
            r#"[["AUS200_otc",1732830108.205,6435.96]]"#,
            r#"[["AEDCNY_otc",1732829668.352,1.89817]]"#,
            r#"[["CADJPY_otc",1732830170.793,109.442]]"#,
        ];
        for item in tests.iter() {
            let val: Value = serde_json::from_str(item)?;
            dbg!(&val);
            let res: UpdateStream = serde_json::from_value(val)?;
            dbg!(res);
            // let descerializer = Deserializer::from_str(item).into_iter::<UpdateStream>();
            // for item in descerializer.into_iter() {

            //     let res = item?;
            //     let time_reparsed = serde_json::to_string(&res)?;
            //     dbg!(time_reparsed);
            //     dbg!(res);
            // }
        }
        Ok(())
    }

    #[test]
    fn test_deserialize_update_history() -> Result<(), Box<dyn Error>> {
        let history_raw = File::open("tests/update_history_new.txt")?;
        let bufreader = BufReader::new(history_raw);
        let history_new: UpdateHistoryNewFast = serde_json::from_reader(bufreader)?;
        dbg!(history_new);

        Ok(())
    }

    #[test]
    fn test_deserialize_load_history1() -> Result<(), Box<dyn Error>> {
        let history_raw = File::open("tests/load_history_period.json")?;
        let bufreader = BufReader::new(history_raw);
        let history_new: LoadHistoryPeriodResult = serde_json::from_reader(bufreader)?;
        dbg!(history_new);

        Ok(())
    }

    #[test]
    fn test_deserialize_load_history2() -> Result<(), Box<dyn Error>> {
        let history_raw = File::open("tests/load_history_period2.json")?;
        let bufreader = BufReader::new(history_raw);
        let history_new: LoadHistoryPeriodResult = serde_json::from_reader(bufreader)?;
        dbg!(history_new);

        Ok(())
    }

    #[test]
    fn test_candle_aggregation() -> Result<(), Box<dyn Error>> {
        // Test aggregation of raw tick data into 60-second candles
        let history_raw = File::open("tests/load_history_period2.json")?;
        let bufreader = BufReader::new(history_raw);
        let history: LoadHistoryPeriodResult = serde_json::from_reader(bufreader)?;
        
        // This should aggregate the raw ticks into 60-second candles
        let aggregated_candles = history.candle_data_with_period(60);
        
        println!("Original data points: {}", history.data.len());
        println!("Aggregated candles: {}", aggregated_candles.len());
        
        // Should have significantly fewer candles after aggregation
        assert!(aggregated_candles.len() < history.data.len());
        assert!(aggregated_candles.len() > 0);
        
        // Check that the first candle has proper OHLC values
        if let Some(first_candle) = aggregated_candles.first() {
            assert!(first_candle.high >= first_candle.low);
            assert!(first_candle.high >= first_candle.open);
            assert!(first_candle.high >= first_candle.close);
            assert!(first_candle.low <= first_candle.open);
            assert!(first_candle.low <= first_candle.close);
            
            println!("First aggregated candle: open={}, high={}, low={}, close={}", 
                     first_candle.open, first_candle.high, first_candle.low, first_candle.close);
        }
        
        Ok(())
    }

    #[test]
    fn test_no_aggregation_for_processed_candles() -> Result<(), Box<dyn Error>> {
        // Test that processed candles are not re-aggregated
        let history_raw = File::open("tests/load_history_period.json")?;
        let bufreader = BufReader::new(history_raw);
        let history: LoadHistoryPeriodResult = serde_json::from_reader(bufreader)?;
        
        let regular_candles = history.candle_data();
        let aggregated_candles = history.candle_data_with_period(60);
        
        // Should be the same since these are already processed candles
        assert_eq!(regular_candles.len(), aggregated_candles.len());
        
        Ok(())
    }
}

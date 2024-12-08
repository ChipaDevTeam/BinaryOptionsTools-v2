use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::update::float_time;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrder {
    asset: String,
    action: String,
    amount: f64,
    is_demo: u32,
    option_type: u32,
    request_id: u64,
    time: u32
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateClosedDeals(pub Vec<Deal>);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SuccessCloseOrder {
    profit: f64,
    deals: Vec<Deal>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UpdateOpenedDeals(pub Vec<Deal>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Deal {
    id: Uuid,
    open_time: String,
    close_time: String,
    #[serde(with = "float_time")]
    open_timestamp: DateTime<Utc>,
    #[serde(with = "float_time")]
    close_timestamp: DateTime<Utc>,
    refund_time: Option<Value>,
    refund_timestamp: Option<Value>,
    uid: u64,
    amount: u64,
    profit: f64,
    percent_profit: i32,
    percent_loss: i32,
    open_price: f64,
    close_price: f64,
    command: i32,
    asset: String,
    is_demo: u32,
    copy_ticket: String,
    open_ms: i32,
    close_ms: Option<i32>,
    option_type: i32,
    is_rollover: Option<bool>,
    is_copy_signal: Option<bool>,
    is_AI: Option<bool>,
    currency: String,
    amount_usd: Option<f64>,
    amount_USD: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessOpenOrder {
    id: Uuid,
    open_time: String,
    close_time: String,
    #[serde(with = "float_time")]
    open_timestamp: DateTime<Utc>,
    #[serde(with = "float_time")]
    close_timestamp: DateTime<Utc>,
    uid: u64,
    is_demo: u32,
    amount: f64,
    profit: f64,
    percent_profit: i32,
    percent_loss: i32,
    open_price: f64,
    copy_ticket: String,
    close_price: f64,
    command: i32,
    asset: String,
    request_id: u64,
    open_ms: i32,
    option_type: i32,
    is_copy_signal: bool,
    currency: String,
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fs::{read_to_string, File}, io::BufReader};

    use crate::pocketoption::{parser::message::WebSocketMessage, types::info::MessageInfo};

    use super::*;

    #[test]
    fn test_descerialize_closed_deals() -> Result<(), Box<dyn Error>> {
        let history_raw = File::open("tests/update_closed_deals.json")?;
        let bufreader = BufReader::new(history_raw);
        let deals: UpdateClosedDeals = serde_json::from_reader(bufreader)?;
        let deals2 = WebSocketMessage::parse_with_context(read_to_string("tests/update_closed_deals.json")?, &MessageInfo::UpdateClosedDeals)?;
        if let WebSocketMessage::UpdateClosedDeals(d) = deals2 {
            assert_eq!(d, deals);
        } else {
            panic!("WebSocketMessage should be UpdateClosedDeals variant")
        }

        Ok(())
    }
    #[test]
    fn test_descerialize_close_order() -> Result<(), Box<dyn Error>> {
        let history_raw = File::open("tests/update_close_order.json")?;
        let bufreader = BufReader::new(history_raw);
        let deals: SuccessCloseOrder = serde_json::from_reader(bufreader)?;
        let deals2 = WebSocketMessage::parse_with_context(read_to_string("tests/update_close_order.json")?, &MessageInfo::SuccesscloseOrder)?;
        if let WebSocketMessage::SuccesscloseOrder(d) = deals2 {
            assert_eq!(d, deals);
        } else {
            panic!("WebSocketMessage should be UpdateClosedDeals variant")
        }
        Ok(())
    }

    #[test]
    fn test_descerialize_open_order() -> Result<(), Box<dyn Error>> {
        let order_raw = File::open("tests/success_open_order.json")?;
        let bufreader = BufReader::new(order_raw);
        let order: SuccessOpenOrder = serde_json::from_reader(bufreader)?;
        dbg!(order);
        Ok(())
    }

    #[test]
    fn test_descerialize_update_opened_deals() -> anyhow::Result<()> {
        let order_raw = File::open("tests/update_opened_deals.json")?;
        let bufreader = BufReader::new(order_raw);
        let order: UpdateOpenedDeals = serde_json::from_reader(bufreader)?;
        dbg!(order);
        Ok(())
    }
}
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Asset information from CloseOption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub main: f64,
    pub source: String, // "AFX" or "CBAT"
}

/// Price data for a single asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPrice {
    pub bid: f64,
    pub ask: f64,
    pub main: f64,
}

/// Real-time price data message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub prices: HashMap<String, AssetPrice>,
    pub timestamp: i64,
}

/// Candle data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: i64,
    pub value: f64,
}

/// Request for 30-minute historical candles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Get30MinRequest {
    pub token: String,
    pub ps_type: String,
    pub public_code: String,
    pub hidden_code: String,
    pub acc_type: String,
    pub pair: String,
    pub contest_type: String,
}

/// Request to place an order
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOrderRequest {
    pub token: String,
    pub time_intervals: String,
    pub amount: f64,
    pub order_type: String,
    pub public_code: String,
    pub hidden_code: String,
    pub acc_type: String,
    pub pair: String,
    pub contest_type: String,
}

/// Order result from CloseOption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResult {
    pub order_id: String,
    pub pair: String,
    pub status: String,
    pub amount: f64,
    pub open_price: f64,
    pub profit: f64,
    pub result: String,
    pub payout: f64,
    pub balance: f64,
    #[serde(default)]
    pub close_price: f64,
    #[serde(default)]
    pub close_time: i64,
    #[serde(default)]
    pub open_time: i64,
}

/// Historical candles result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Get30MinResult {
    pub candles: Vec<Candle>,
    pub pair: String,
}

/// Outgoing message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum Outgoing {
    Get30Min(Get30MinRequest),
    SetOrder(SetOrderRequest),
    Ping,
}

/// Incoming subscription events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum SubscriptionEvent {
    PriceData(PriceData),
    Get30MinResult(Get30MinResult),
    SetOrderResult(OrderResult),
}

/// Raw Socket.IO message frame
#[derive(Debug, Clone)]
pub struct SocketIoFrame {
    pub message_type: SocketIoMessageType,
    pub namespace: Option<String>,
    pub data: String,
}

/// Socket.IO message types (EIO=3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketIoMessageType {
    Connect = 0,
    Disconnect = 1,
    Event = 2,
    Ack = 3,
    Error = 4,
    BinaryEvent = 5,
    BinaryAck = 6,
    ConnectError = 7,
}

impl SocketIoMessageType {
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(SocketIoMessageType::Connect),
            1 => Some(SocketIoMessageType::Disconnect),
            2 => Some(SocketIoMessageType::Event),
            3 => Some(SocketIoMessageType::Ack),
            4 => Some(SocketIoMessageType::Error),
            5 => Some(SocketIoMessageType::BinaryEvent),
            6 => Some(SocketIoMessageType::BinaryAck),
            7 => Some(SocketIoMessageType::ConnectError),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// Socket.IO EIO=3 frame parser
pub mod socket_io {
    use crate::closeoption::error::CloseOptionError;
    pub use crate::closeoption::types::{SocketIoFrame, SocketIoMessageType};
    pub fn parse_frame(data: &str) -> Result<SocketIoFrame, CloseOptionError> {
        if data.is_empty() {
            return Err(CloseOptionError::Parse("Empty frame".to_string()));
        }
        
        // Socket.IO EIO=3 frames can have combined prefixes like "42" (Engine.IO message + Socket.IO event)
        // The first digit is Engine.IO packet type, second is Socket.IO packet type
        let chars: Vec<char> = data.chars().collect();
        if chars.len() < 1 {
            return Err(CloseOptionError::Parse("Empty frame".to_string()));
        }
        
        let first_digit = chars[0].to_digit(10)
            .ok_or_else(|| CloseOptionError::Parse(format!("Invalid first character: {}", chars[0])))?;
        
        // Check if it's a combined prefix (e.g., "42" = Engine.IO message + Socket.IO event)
        // The first digit is Engine.IO packet type, second is Socket.IO packet type
        let (socket_io_type, rest_start) = if chars.len() >= 2 {
            if let Some(second_digit) = chars[1].to_digit(10) {
                // Combined prefix like "42"
                (second_digit, 2)
            } else {
                // Single digit prefix
                (first_digit, 1)
            }
        } else {
            // Single digit only
            (first_digit, 1)
        };
        
        let msg_type = SocketIoMessageType::from_u8(socket_io_type as u8)
            .ok_or_else(|| CloseOptionError::Parse(format!("Invalid Socket.IO message type: {}", socket_io_type)))?;
        
        let rest = &data[rest_start..];

        // Check for namespace (starts with '/')
        let (namespace, payload) = if rest.starts_with('/') {
            let end = rest.find(',').or_else(|| rest.find('[')).unwrap_or(rest.len());
            let ns = rest[1..end].to_string();
            (Some(ns), &rest[end..])
        } else {
            (None, rest)
        };

        Ok(SocketIoFrame {
            message_type: msg_type,
            namespace,
            data: payload.to_string(),
        })
    }

    /// Encode a Socket.IO EIO=3 frame
    pub fn encode_frame(msg_type: SocketIoMessageType, namespace: Option<&str>, data: &str) -> String {
        let mut result = String::new();
        result.push(char::from_digit(msg_type.as_u8() as u32, 10).unwrap());

        if let Some(ns) = namespace {
            result.push('/');
            result.push_str(ns);
            result.push(',');
        }

        result.push_str(data);
        result
    }

    /// Create a probe packet (2probe)
    pub fn probe() -> String {
        "2probe".to_string()
    }

    /// Create an upgrade packet (5)
    pub fn upgrade() -> String {
        "5".to_string()
    }

    /// Create a ping packet (2)
    pub fn ping() -> String {
        "2".to_string()
    }

    /// Create an event packet (42["event", data])
    pub fn event(event: &str, data: &str) -> String {
        format!("42[{}, {}]", serde_json::to_string(event).unwrap(), data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_probe() {
        let frame = socket_io::parse_frame("2probe").unwrap();
        assert_eq!(frame.message_type, SocketIoMessageType::Event);
        assert_eq!(frame.data, "probe");
    }

    #[test]
    fn test_parse_upgrade() {
        let frame = socket_io::parse_frame("5").unwrap();
        assert_eq!(frame.message_type, SocketIoMessageType::BinaryEvent);
        assert_eq!(frame.data, "");
    }

    #[test]
    fn test_parse_ping() {
        let frame = socket_io::parse_frame("2").unwrap();
        assert_eq!(frame.message_type, SocketIoMessageType::Event);
        assert_eq!(frame.data, "");
    }

    #[test]
    fn test_parse_event() {
        let frame = socket_io::parse_frame(r#"42["priceData",{"prices":{}}]"#).unwrap();
        assert_eq!(frame.message_type, SocketIoMessageType::Event);
        assert!(frame.data.contains("priceData"));
    }

    #[test]
    fn test_encode_probe() {
        assert_eq!(socket_io::probe(), "2probe");
    }

    #[test]
    fn test_encode_upgrade() {
        assert_eq!(socket_io::upgrade(), "5");
    }

    #[test]
    fn test_encode_ping() {
        assert_eq!(socket_io::ping(), "2");
    }

    #[test]
    fn test_encode_event() {
        let encoded = socket_io::event("priceData", r#"{"prices":{}}"#);
        assert!(encoded.starts_with("42[\"priceData\""));
    }
}
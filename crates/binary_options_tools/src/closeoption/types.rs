use serde::{Deserialize, Serialize};
use super::error::CloseOptionError;
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
    #[serde(alias = "timeStamp")]
    pub timestamp: i64,
    pub value: f64,
}

/// Request for 30-minute historical candles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Get30MinRequest {
    #[serde(rename = "_token")]
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
pub struct Get30MinResult {
    pub price: Vec<Candle>,
}

/// Outgoing message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Outgoing {
    Get30Min(Get30MinRequest),
    SetOrder(SetOrderRequest),
    Ping,
}

impl Outgoing {
    pub fn event_name(&self) -> &'static str {
        match self {
            Outgoing::Get30Min(_) => "get30Min",
            Outgoing::SetOrder(_) => "setOrder",
            Outgoing::Ping => "ping",
        }
    }

    pub fn as_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}


/// Incoming subscription events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SubscriptionEvent {
    PriceData(PriceData),
    Get30MinResult(Get30MinResult),
    SetOrderResult(OrderResult),
    Error(String),
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
    // Engine.IO packet types
    EngineOpen = 10,
    EngineClose = 11,
    EnginePing = 12,
    EnginePong = 13,
    EngineMessage = 14,
    EngineUpgrade = 15,
    EngineNoop = 16,
    // Socket.IO packet types (when Engine.IO type is Message=4)
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
            10 => Some(SocketIoMessageType::EngineOpen),
            11 => Some(SocketIoMessageType::EngineClose),
            12 => Some(SocketIoMessageType::EnginePing),
            13 => Some(SocketIoMessageType::EnginePong),
            14 => Some(SocketIoMessageType::EngineMessage),
            15 => Some(SocketIoMessageType::EngineUpgrade),
            16 => Some(SocketIoMessageType::EngineNoop),
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
        
        let chars: Vec<char> = data.chars().collect();
        if chars.is_empty() {
            return Err(CloseOptionError::Parse("Empty frame".to_string()));
        }
        
        // First digit is Engine.IO packet type
        let engine_type = chars[0].to_digit(10)
            .ok_or_else(|| CloseOptionError::Parse(format!("Invalid first character: {}", chars[0])))?;
        
        match engine_type {
            // Engine.IO ping (2)
            2 => {
                let rest = &data[1..];
                Ok(SocketIoFrame {
                    message_type: SocketIoMessageType::EnginePing,
                    namespace: None,
                    data: rest.to_string(),
                })
            }
            // Engine.IO pong (3)
            3 => {
                let rest = &data[1..];
                Ok(SocketIoFrame {
                    message_type: SocketIoMessageType::EnginePong,
                    namespace: None,
                    data: rest.to_string(),
                })
            }
            // Engine.IO upgrade (5)
            5 => {
                Ok(SocketIoFrame {
                    message_type: SocketIoMessageType::EngineUpgrade,
                    namespace: None,
                    data: String::new(),
                })
            }
            // Engine.IO message (4) - contains Socket.IO packet
            4 => {
                let rest = &data[1..];
                if rest.is_empty() {
                    return Err(CloseOptionError::Parse("Empty Socket.IO payload".to_string()));
                }
                let socket_io_chars: Vec<char> = rest.chars().collect();
                let socket_io_type = socket_io_chars[0].to_digit(10)
                    .ok_or_else(|| CloseOptionError::Parse(format!("Invalid Socket.IO type character: {}", socket_io_chars[0])))?;
                
                let msg_type = SocketIoMessageType::from_u8(socket_io_type as u8)
                    .ok_or_else(|| CloseOptionError::Parse(format!("Invalid Socket.IO message type: {}", socket_io_type)))?;
                
                let payload = &rest[1..];
                
                // Check for namespace (starts with '/')
                let (namespace, payload) = if payload.starts_with('/') {
                    let end = payload.find(',').or_else(|| payload.find('[')).unwrap_or(payload.len());
                    let ns = payload[1..end].to_string();
                    (Some(ns), &payload[end..])
                } else {
                    (None, payload)
                };
                
                Ok(SocketIoFrame {
                    message_type: msg_type,
                    namespace,
                    data: payload.to_string(),
                })
            }
            // Engine.IO open (0), close (1), noop (6)
            0 => Ok(SocketIoFrame {
                message_type: SocketIoMessageType::EngineOpen,
                namespace: None,
                data: data[1..].to_string(),
            }),
            1 => Ok(SocketIoFrame {
                message_type: SocketIoMessageType::EngineClose,
                namespace: None,
                data: data[1..].to_string(),
            }),
            6 => Ok(SocketIoFrame {
                message_type: SocketIoMessageType::EngineNoop,
                namespace: None,
                data: data[1..].to_string(),
            }),
            _ => Err(CloseOptionError::Parse(format!("Unknown Engine.IO packet type: {}", engine_type))),
        }
    }

    /// Encode a Socket.IO EIO=3 frame
    pub fn encode_frame(msg_type: SocketIoMessageType, namespace: Option<&str>, data: &str) -> String {
        let mut result = String::new();
        let code = msg_type.as_u8();
        if code < 8 {
            // Socket.IO packet: prepend Engine.IO Message prefix '4'
            result.push('4');
            result.push(char::from_digit(code as u32, 10).unwrap());
            if let Some(ns) = namespace {
                result.push('/');
                result.push_str(ns);
                result.push(',');
            }
        } else {
            // Engine.IO packet types 10-16: emit single-digit wire code
            let wire_code = match msg_type {
                SocketIoMessageType::EngineOpen => '0',
                SocketIoMessageType::EngineClose => '1',
                SocketIoMessageType::EnginePing => '2',
                SocketIoMessageType::EnginePong => '3',
                SocketIoMessageType::EngineMessage => '4',
                SocketIoMessageType::EngineUpgrade => '5',
                SocketIoMessageType::EngineNoop => '6',
                _ => unreachable!(),
            };
            result.push(wire_code);
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
        assert_eq!(frame.message_type, SocketIoMessageType::EnginePing);
        assert_eq!(frame.data, "probe");
    }

    #[test]
    fn test_parse_upgrade() {
        let frame = socket_io::parse_frame("5").unwrap();
        assert_eq!(frame.message_type, SocketIoMessageType::EngineUpgrade);
        assert_eq!(frame.data, "");
    }

    #[test]
    fn test_parse_ping() {
        let frame = socket_io::parse_frame("2").unwrap();
        assert_eq!(frame.message_type, SocketIoMessageType::EnginePing);
        assert_eq!(frame.data, "");
    }

    #[test]
    fn test_parse_pong() {
        let frame = socket_io::parse_frame("3probe").unwrap();
        assert_eq!(frame.message_type, SocketIoMessageType::EnginePong);
        assert_eq!(frame.data, "probe");
    }

    #[test]
    fn test_parse_event() {
        let frame = socket_io::parse_frame("42[\"priceData\",{\"prices\":{}}]").unwrap();
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

    #[test]
    fn test_encode_engine_ping() {
        let encoded = socket_io::encode_frame(SocketIoMessageType::EnginePing, None, "probe");
        assert_eq!(encoded, "2probe");
    }

    #[test]
    fn test_encode_socket_event() {
        let encoded = socket_io::encode_frame(SocketIoMessageType::Event, None, r#"["priceData",{}]"#);
        assert_eq!(encoded, r#"42["priceData",{}]"#);
    }
}
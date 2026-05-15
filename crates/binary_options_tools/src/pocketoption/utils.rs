use std::sync::atomic::{AtomicU64, Ordering};

use binary_options_tools_core::connector::{ConnectorError, ConnectorResult};
use binary_options_tools_core::error::{CoreError, CoreResult};
use binary_options_tools_core::reimports::{
    connect_async_tls_with_config, generate_key, Connector, MaybeTlsStream, Request,
    WebSocketStream,
};
use std::sync::OnceLock;
use std::time::Duration as StdDuration;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    ssid::Ssid,
};
use crate::utils::init_crypto_provider;
use serde_json::Value;
use tokio::net::TcpStream;

use url::Url;

static CONNECTOR: OnceLock<Connector> = OnceLock::new();

fn get_connector() -> CoreResult<&'static Connector> {
    if let Some(connector) = CONNECTOR.get() {
        return Ok(connector);
    }

    let mut root_store = rustls::RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs().certs;
    if certs.is_empty() {
        return Err(CoreError::Connection(ConnectorError::Custom(
            "Could not load any native certificates".to_string(),
        )));
    }
    for cert in certs {
        root_store.add(cert).ok();
    }
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = Connector::Rustls(std::sync::Arc::new(tls_config));
    let _ = CONNECTOR.set(connector);
    CONNECTOR
        .get()
        .ok_or_else(|| CoreError::Other("Connector not initialized".into()))
}

const IP_PROVIDERS: &[&str] = &[
    "https://i.pn/json/",
    "https://ip.pn/json/",
    "https://ipv4.myip.coffee",
    "https://api.ipify.org?format=json",
    "https://httpbin.org/ip",
    "https://ifconfig.co/json",
    "https://ipapi.co/",
    "https://ipwho.is/",
];
const EARTH_RADIUS_KM: f64 = 6371.0;

/// Threshold for distinguishing millisecond timestamps from second timestamps.
/// 1_000_000_000_000.0 (~year 33658 in seconds) is far beyond any valid second-based
/// Unix timestamp, so any value above this is treated as milliseconds.
const MS_THRESHOLD: f64 = 1_000_000_000_000.0;

/// Normalizes a raw timestamp value to Unix seconds (i64).
///
/// Handles both second-based and millisecond-based timestamps automatically.
/// Uses rounding (not truncation) to avoid off-by-one-second errors.
///
/// # Arguments
/// * `raw` - Raw timestamp as f64 (either seconds or milliseconds)
///
/// # Returns
/// Normalized Unix timestamp in seconds as i64
#[inline]
pub fn normalize_timestamp(raw: f64) -> i64 {
    if raw > MS_THRESHOLD {
        (raw / 1000.0).trunc() as i64
    } else {
        raw.trunc() as i64
    }
}

static INDEX_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn get_index() -> PocketResult<u64> {
    Ok(INDEX_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub async fn get_user_location(ip_address: &str) -> PocketResult<(f64, f64)> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(2))
        .build()
        .map_err(|e| PocketError::General(format!("Failed to build HTTP client: {e}")))?;

    for url in IP_PROVIDERS {
        let target = if url.contains("ipapi.co") {
            format!("{}{}/json/", url, ip_address)
        } else if url.contains("ipwho.is") || url.contains("i.pn") || url.contains("ip.pn") {
            format!("{}{}", url, ip_address)
        } else {
            continue;
        };

        tracing::debug!(target: "PocketUtils", "Trying geo provider: {}", target);
        if let Ok(response) = client.get(&target).send().await {
            if let Ok(json) = response.json::<Value>().await {
                let lat = json["lat"].as_f64().or_else(|| json["latitude"].as_f64());
                let lon = json["lon"].as_f64().or_else(|| json["longitude"].as_f64());

                if let (Some(lat), Some(lon)) = (lat, lon) {
                    tracing::debug!(target: "PocketUtils", "Found location via {}: {}, {}", target, lat, lon);
                    return Ok((lat, lon));
                }
            }
        }
    }

    tracing::warn!(target: "PocketUtils", "All geo providers failed for IP {}. Using fallback location.", ip_address);
    // Default or fallback location (e.g. US Central) if all fail
    Ok((37.0902, -95.7129))
}

pub fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    // Haversine formula to calculate distance between two coordinates
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();

    let a = dlat.sin().powi(2) + lat1.cos() * lat2.cos() * dlon.sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_KM * c
}

pub async fn get_public_ip() -> PocketResult<String> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(2))
        .build()
        .map_err(|e| PocketError::General(format!("Failed to build HTTP client: {e}")))?;

    for url in IP_PROVIDERS {
        let target = url.to_string();
        tracing::debug!(target: "PocketUtils", "Trying IP provider: {}", target);
        match client.get(&target).send().await {
            Ok(response) => {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(ip) = json["ip"]
                        .as_str()
                        .or_else(|| json["query"].as_str())
                        .or_else(|| json["origin"].as_str())
                    {
                        tracing::debug!(target: "PocketUtils", "Found public IP via {}: {}", target, ip);
                        return Ok(ip.to_string());
                    }
                }
            }
            Err(e) => {
                tracing::debug!(target: "PocketUtils", "Provider {} failed: {}", target, e);
                continue;
            }
        }
    }

    Err(PocketError::General(
        "Failed to retrieve public IP from any provider".into(),
    ))
}

pub async fn try_connect(
    ssid: Ssid,
    url: String,
) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    init_crypto_provider();
    let connector = get_connector().map_err(|e| ConnectorError::Core(e.to_string()))?;

    let user_agent = ssid.user_agent();

    let t_url = Url::parse(&url).map_err(|e| ConnectorError::UrlParsing(e.to_string()))?;
    let host = t_url
        .host_str()
        .ok_or(ConnectorError::UrlParsing("Host not found".into()))?;

    tracing::debug!(target: "PocketConnect", "Connecting to {} with UA: {} and Origin: https://pocketoption.com", host, user_agent);

    let request = Request::builder()
        .uri(t_url.to_string())
        .header("Host", host)
        .header("User-Agent", user_agent)
        .header("Origin", "https://pocketoption.com")
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13")
        .body(())
        .map_err(|e| ConnectorError::HttpRequestBuild(e.to_string()))?;

    let (ws, _) = tokio::time::timeout(
        StdDuration::from_secs(10),
        connect_async_tls_with_config(request, None, false, Some(connector.clone())),
    )
    .await
    .map_err(|_| ConnectorError::Timeout)?
    .map_err(|e| ConnectorError::Custom(e.to_string()))?;
    Ok(ws)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketIoMessageType {
    Connect,      // 0
    Disconnect,   // 1
    Event,        // 2
    Ack,          // 3
    ConnectError, // 4
    BinaryEvent,  // 5
    BinaryAck,    // 6
}

impl SocketIoMessageType {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '0' => Some(Self::Connect),
            '1' => Some(Self::Disconnect),
            '2' => Some(Self::Event),
            '3' => Some(Self::Ack),
            '4' => Some(Self::ConnectError),
            '5' => Some(Self::BinaryEvent),
            '6' => Some(Self::BinaryAck),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SocketIoFrame {
    pub engine_type: char,                         // e.g. '4'
    pub message_type: Option<SocketIoMessageType>, // e.g. '2'
    pub namespace: Option<String>,
    pub id: Option<u64>,
    pub data: Option<String>,
}

impl SocketIoFrame {
    /// Parses a raw Socket.IO string frame (e.g. "42[\"event\",{...}]").
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars().peekable();

        // 1. Engine.IO packet type (mandatory)
        let engine_type = chars.next()?;

        // 2. Socket.IO message type (optional for some Engine.IO types like Ping/Pong)
        let message_type = chars
            .peek()
            .and_then(|&c| SocketIoMessageType::from_char(c));
        if message_type.is_some() {
            chars.next();
        }

        let mut remaining: String = chars.collect();

        // 3. Namespace (optional, starts with /)
        let mut namespace = None;
        if remaining.starts_with('/') {
            if let Some(comma_pos) = remaining.find(',') {
                namespace = Some(remaining[..comma_pos].to_string());
                remaining = remaining[comma_pos + 1..].to_string();
            }
        }

        // 4. For BinaryEvent/BinaryAck: attachment count (digits followed by '-')
        //    For other types: ack ID (optional, numeric, only if not followed by '-')
        let mut id = None;
        let is_binary = matches!(
            message_type,
            Some(SocketIoMessageType::BinaryEvent) | Some(SocketIoMessageType::BinaryAck)
        );

        let id_digits: String = remaining
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !id_digits.is_empty() {
            if is_binary {
                // Binary attachment count - skip the digits and the following '-' separator
                remaining = remaining[id_digits.len()..].to_string();
                if remaining.starts_with('-') {
                    remaining = remaining[1..].to_string();
                }
            } else {
                // For non-binary types, only treat digits as ack ID when NOT followed by '-'
                // (which indicates binary attachment syntax, e.g. "451-[...]")
                let after_digits = remaining.chars().nth(id_digits.len());
                if after_digits == Some('-') {
                    // Digits are part of binary/attachment syntax, not an ack ID;
                    // leave remaining untouched so the payload is preserved intact.
                } else {
                    id = id_digits.parse().ok();
                    remaining = remaining[id_digits.len()..].to_string();
                }
            }
        }

        // 5. Data (optional, usually starts with [ or {)
        let data = if remaining.is_empty() {
            None
        } else {
            Some(remaining)
        };

        Some(Self {
            engine_type,
            message_type,
            namespace,
            id,
            data,
        })
    }

    /// Extracts the event name and payload from the data array.
    /// Supports multiple formats:
    /// 1. Standard: ["eventName", {payload}]
    /// 2. Nested: [["eventName", {payload}]]
    /// 3. Multi-event: ["eventName", {payload}, "anotherEvent", ...] (returns first)
    pub fn extract_event(&self) -> Option<(String, serde_json::Value)> {
        let data = self.data.as_ref()?;
        let value: serde_json::Value = serde_json::from_str(data).ok()?;

        if let Some(arr) = value.as_array() {
            if arr.is_empty() {
                return None;
            }

            // Case 2: Nested array [[...]]
            if let Some(inner_arr) = arr[0].as_array() {
                if inner_arr.len() >= 2 {
                    if let Some(event_name) = inner_arr[0].as_str() {
                        return Some((event_name.to_string(), inner_arr[1].clone()));
                    }
                }
            }

            // Case 1 & 3: Standard or Multi-event
            if arr.len() >= 2 {
                if let Some(event_name) = arr[0].as_str() {
                    return Some((event_name.to_string(), arr[1].clone()));
                }
            }
        }
        None
    }
}
pub mod unix_timestamp {

    use chrono::{DateTime, Utc};

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(date.timestamp())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        let timestamp = if let Some(i) = value.as_i64() {
            i
        } else if let Some(f) = value.as_f64() {
            f.trunc() as i64
        } else {
            return Err(serde::de::Error::custom(
                "Error parsing timestamp: expected number",
            ));
        };

        DateTime::from_timestamp(timestamp, 0).ok_or(serde::de::Error::custom(
            "Error parsing timestamp to DateTime",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_socket_io_frame_parsing() {
        // Standard format
        let frame = SocketIoFrame::parse("42[\"event\",{\"data\":1}]").unwrap();
        let (event, payload) = frame.extract_event().unwrap();
        assert_eq!(event, "event");
        assert_eq!(payload, json!({"data":1}));

        // Nested array format
        let frame = SocketIoFrame::parse("42[[\"nestedEvent\",{\"val\":2}]]").unwrap();
        let (event, payload) = frame.extract_event().unwrap();
        assert_eq!(event, "nestedEvent");
        assert_eq!(payload, json!({"val":2}));

        // Multi-event format (should return first)
        let frame = SocketIoFrame::parse("42[\"firstEvent\",{\"a\":1},\"secondEvent\",{\"b\":2}]").unwrap();
        let (event, payload) = frame.extract_event().unwrap();
        assert_eq!(event, "firstEvent");
        assert_eq!(payload, json!({"a":1}));
    }
}

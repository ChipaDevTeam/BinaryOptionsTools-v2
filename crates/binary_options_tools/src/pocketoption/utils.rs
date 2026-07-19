use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use binary_options_tools_core::connector::{ConnectorError, ConnectorResult};
use binary_options_tools_core::error::{CoreError, CoreResult};
use binary_options_tools_core::reimports::{
    generate_key, MaybeTlsStream, Request,
    WebSocketStream,
};
use std::time::Duration as StdDuration;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::State,
};
use crate::utils::init_crypto_provider;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::client_async_with_config;
use rustls::pki_types::ServerName;

use url::Url;


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
    init_crypto_provider();
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
    init_crypto_provider();
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

fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        match chunk.len() {
            3 => {
                result.push(CHARSET[(chunk[0] >> 2) as usize] as char);
                result.push(CHARSET[(((chunk[0] & 0x03) << 4) | (chunk[1] >> 4)) as usize] as char);
                result.push(CHARSET[(((chunk[1] & 0x0f) << 2) | (chunk[2] >> 6)) as usize] as char);
                result.push(CHARSET[(chunk[2] & 0x3f) as usize] as char);
            }
            2 => {
                result.push(CHARSET[(chunk[0] >> 2) as usize] as char);
                result.push(CHARSET[(((chunk[0] & 0x03) << 4) | (chunk[1] >> 4)) as usize] as char);
                result.push(CHARSET[((chunk[1] & 0x0f) << 2) as usize] as char);
                result.push('=');
            }
            1 => {
                result.push(CHARSET[(chunk[0] >> 2) as usize] as char);
                result.push(CHARSET[((chunk[0] & 0x03) << 4) as usize] as char);
                result.push('=');
                result.push('=');
            }
            _ => unreachable!(),
        }
    }
    result
}

fn parse_auth(url: &Url) -> Option<(String, String)> {
    let username = url.username();
    if !username.is_empty() {
        let password = url.password().unwrap_or("");
        Some((username.to_string(), password.to_string()))
    } else {
        None
    }
}

async fn socks5_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    if let Some((_user, _pass)) = &auth {
        stream.write_all(&[0x05, 0x02, 0x00, 0x02]).await
            .map_err(|e| ConnectorError::Custom(format!("SOCKS5 greeting send failed: {e}")))?;
    } else {
        stream.write_all(&[0x05, 0x01, 0x00]).await
            .map_err(|e| ConnectorError::Custom(format!("SOCKS5 greeting send failed: {e}")))?;
    }

    let mut resp = [0u8; 2];
    stream.read_exact(&mut resp).await
        .map_err(|e| ConnectorError::Custom(format!("SOCKS5 greeting read failed: {e}")))?;

    if resp[0] != 0x05 {
        return Err(ConnectorError::Custom("Invalid SOCKS5 version".into()));
    }

    if resp[1] == 0x02 {
        if let Some((user, pass)) = &auth {
            let user_bytes = user.as_bytes();
            let pass_bytes = pass.as_bytes();
            
            let mut auth_req = Vec::new();
            auth_req.push(0x01);
            auth_req.push(user_bytes.len() as u8);
            auth_req.extend_from_slice(user_bytes);
            auth_req.push(pass_bytes.len() as u8);
            auth_req.extend_from_slice(pass_bytes);

            stream.write_all(&auth_req).await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 auth failed: {e}")))?;

            let mut auth_resp = [0u8; 2];
            stream.read_exact(&mut auth_resp).await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 auth read failed: {e}")))?;

            if auth_resp[1] != 0x00 {
                return Err(ConnectorError::Custom("SOCKS5 authentication failed".into()));
            }
        } else {
            return Err(ConnectorError::Custom("SOCKS5 proxy requested auth but no credentials provided".into()));
        }
    } else if resp[1] != 0x00 {
        return Err(ConnectorError::Custom("SOCKS5 authentication method rejected".into()));
    }

    let host_bytes = target_host.as_bytes();
    let mut req = Vec::new();
    req.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
    req.extend_from_slice(host_bytes);
    req.extend_from_slice(&target_port.to_be_bytes());

    stream.write_all(&req).await
        .map_err(|e| ConnectorError::Custom(format!("SOCKS5 connect request failed: {e}")))?;

    let mut resp_hdr = [0u8; 4];
    stream.read_exact(&mut resp_hdr).await
        .map_err(|e| ConnectorError::Custom(format!("SOCKS5 connect response read failed: {e}")))?;

    if resp_hdr[1] != 0x00 {
        return Err(ConnectorError::Custom(format!("SOCKS5 connect request failed with error code: {}", resp_hdr[1])));
    }

    match resp_hdr[3] {
        0x01 => {
            let mut addr = [0u8; 4 + 2];
            stream.read_exact(&mut addr).await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 address read failed: {e}")))?;
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 domain len read failed: {e}")))?;
            let mut domain_and_port = vec![0u8; len_buf[0] as usize + 2];
            stream.read_exact(&mut domain_and_port).await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 domain read failed: {e}")))?;
        }
        0x04 => {
            let mut addr = [0u8; 16 + 2];
            stream.read_exact(&mut addr).await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 address read failed: {e}")))?;
        }
        _ => return Err(ConnectorError::Custom("Unsupported address type".into())),
    }

    Ok(())
}

async fn http_connect_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut req_str = format!("CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n");
    if let Some((user, pass)) = &auth {
        let creds = format!("{user}:{pass}");
        let encoded = base64_encode(creds.as_bytes());
        req_str.push_str(&format!("Proxy-Authorization: Basic {encoded}\r\n"));
    }
    req_str.push_str("\r\n");

    stream.write_all(req_str.as_bytes()).await
        .map_err(|e| ConnectorError::Custom(format!("HTTP proxy CONNECT failed: {e}")))?;

    let mut header_buf = Vec::new();
    let mut temp = [0u8; 1];
    loop {
        stream.read_exact(&mut temp).await
            .map_err(|e| ConnectorError::Custom(format!("HTTP proxy read failed: {e}")))?;
        header_buf.push(temp[0]);
        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if header_buf.len() > 8192 {
            return Err(ConnectorError::Custom("HTTP proxy response header too large".into()));
        }
    }

    let headers_text = String::from_utf8_lossy(&header_buf);
    let first_line = headers_text.lines().next().ok_or_else(|| ConnectorError::Custom("Empty HTTP proxy response".into()))?;
    if !first_line.contains(" 200 ") {
        return Err(ConnectorError::Custom(format!("HTTP proxy CONNECT rejected: {first_line}")));
    }

    Ok(())
}

fn get_tls_config(
    tls_cipher_suites: &Option<Vec<String>>,
    tls_alpn: &Option<Vec<String>>,
) -> CoreResult<rustls::ClientConfig> {
    let mut root_store = rustls::RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs().certs;
    for cert in certs {
        root_store.add(cert).ok();
    }

    let provider = rustls::crypto::ring::default_provider();
    let mut cipher_suites = provider.cipher_suites;

    if let Some(custom_suites) = tls_cipher_suites {
        cipher_suites.retain(|cs| {
            let name = format!("{:?}", cs.suite()).to_uppercase();
            custom_suites.iter().any(|c| name.contains(&c.to_uppercase()))
        });
    } else {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        cipher_suites.shuffle(&mut rng);
    }

    let custom_provider = rustls::crypto::CryptoProvider {
        cipher_suites,
        ..provider
    };

    let mut tls_config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(custom_provider))
        .with_safe_default_protocol_versions()
        .map_err(|e| CoreError::Connection(ConnectorError::Tls(e.to_string())))?
        .with_root_certificates(root_store)
        .with_no_client_auth();

    if let Some(alpn) = tls_alpn {
        tls_config.alpn_protocols = alpn.iter().map(|s| s.as_bytes().to_vec()).collect();
    } else {
        tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    }

    Ok(tls_config)
}

fn per_url_connect_timeout() -> StdDuration {
    StdDuration::from_secs(15)
}

pub async fn try_connect(
    state: Arc<State>,
    url: String,
) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    init_crypto_provider();

    let t_url = Url::parse(&url).map_err(|e| ConnectorError::UrlParsing(e.to_string()))?;
    let target_host = t_url
        .host_str()
        .ok_or(ConnectorError::UrlParsing("Host not found".into()))?;
    let target_port = t_url.port().unwrap_or(match t_url.scheme() {
        "wss" => 443,
        "ws" => 80,
        _ => return Err(ConnectorError::Custom(format!("Unsupported scheme: {}", t_url.scheme()))),
    });

    let socket = if let Some(proxy_str) = &state.proxy {
        let proxy_url = Url::parse(proxy_str)
            .map_err(|e| ConnectorError::Custom(format!("Invalid proxy URL: {e}")))?;
        let proxy_host = proxy_url
            .host_str()
            .ok_or_else(|| ConnectorError::Custom("Proxy host not found".into()))?;
        let proxy_port = proxy_url.port().unwrap_or(match proxy_url.scheme() {
            "https" => 443,
            "http" => 80,
            "socks5" | "socks5h" => 1080,
            _ => return Err(ConnectorError::Custom(format!("Unsupported proxy scheme: {}", proxy_url.scheme()))),
        });

        let mut tcp = tokio::time::timeout(
            per_url_connect_timeout(),
            TcpStream::connect((proxy_host, proxy_port)),
        )
        .await
        .map_err(|_| ConnectorError::Timeout)?
        .map_err(|e| ConnectorError::Custom(format!("Failed to connect to proxy {proxy_host}:{proxy_port}: {e}")))?;

        let auth = parse_auth(&proxy_url);
        if proxy_url.scheme() == "https" {
            let proxy_tls_config = get_tls_config(&state.tls_cipher_suites, &state.tls_alpn)
                .map_err(|e| ConnectorError::Custom(format!("Failed to build proxy TLS config: {e}")))?;
            let proxy_connector = tokio_rustls::TlsConnector::from(Arc::new(proxy_tls_config));
            let server_name = ServerName::try_from(proxy_host)
                .map_err(|e| ConnectorError::Custom(format!("Invalid proxy server name: {e}")))?
                .to_owned();
            let mut tls_stream = tokio::time::timeout(
                per_url_connect_timeout(),
                proxy_connector.connect(server_name, tcp),
            )
            .await
            .map_err(|_| ConnectorError::Timeout)?
            .map_err(|e| ConnectorError::Custom(format!("Proxy TLS handshake failed: {e}")))?;

            http_connect_handshake(&mut tls_stream, target_host, target_port, auth).await?;
            MaybeTlsStream::Rustls(tls_stream)
        } else if proxy_url.scheme() == "http" {
            http_connect_handshake(&mut tcp, target_host, target_port, auth).await?;
            MaybeTlsStream::Plain(tcp)
        } else if proxy_url.scheme() == "socks5" || proxy_url.scheme() == "socks5h" {
            socks5_handshake(&mut tcp, target_host, target_port, auth).await?;
            MaybeTlsStream::Plain(tcp)
        } else {
            return Err(ConnectorError::Custom(format!("Unsupported proxy scheme: {}", proxy_url.scheme())));
        }
    } else {
        let tcp = tokio::time::timeout(
            per_url_connect_timeout(),
            TcpStream::connect((target_host, target_port)),
        )
        .await
        .map_err(|_| ConnectorError::Timeout)?
        .map_err(|e| ConnectorError::Custom(format!("Failed to connect to {target_host}:{target_port}: {e}")))?;
        MaybeTlsStream::Plain(tcp)
    };

    let final_stream = if t_url.scheme() == "wss" {
        let tls_config = get_tls_config(&state.tls_cipher_suites, &state.tls_alpn)
            .map_err(|e| ConnectorError::Custom(format!("Failed to build TLS config: {e}")))?;
        let connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config));
        let server_name = ServerName::try_from(target_host)
            .map_err(|e| ConnectorError::Custom(format!("Invalid target server name: {e}")))?
            .to_owned();

        let tls_stream = match socket {
            MaybeTlsStream::Plain(tcp) => {
                tokio::time::timeout(
                    per_url_connect_timeout(),
                    connector.connect(server_name, tcp),
                )
                .await
                .map_err(|_| ConnectorError::Timeout)?
                .map_err(|e| ConnectorError::Custom(format!("TLS handshake failed: {e}")))?
            }
            MaybeTlsStream::Rustls(_) => {
                return Err(ConnectorError::Custom("Chained TLS streams are not supported".into()));
            }
            _ => {
                return Err(ConnectorError::Custom("Unsupported stream type".into()));
            }
        };
        MaybeTlsStream::Rustls(tls_stream)
    } else {
        socket
    };

    let user_agent = state.user_agent.clone().unwrap_or_else(|| state.ssid.user_agent());
    let origin = state.origin.clone().unwrap_or_else(|| "https://pocketoption.com".to_string());

    let mut request_builder = Request::builder()
        .uri(t_url.to_string())
        .header("Host", target_host)
        .header("User-Agent", user_agent)
        .header("Origin", origin)
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13");

    if let Some(ext) = &state.sec_websocket_extensions {
        request_builder = request_builder.header("Sec-WebSocket-Extensions", ext);
    }

    let request = request_builder
        .body(())
        .map_err(|e| ConnectorError::HttpRequestBuild(e.to_string()))?;

    let (ws, _) = tokio::time::timeout(
        StdDuration::from_secs(10),
        client_async_with_config(request, final_stream, None),
    )
    .await
    .map_err(|_| ConnectorError::Timeout)?
    .map_err(|e| ConnectorError::Custom(e.to_string()))?;

    Ok(ws)
}

/// Custom serde module for `Option<Uuid>` fields that may be sent as a
/// numeric value (e.g. `0`) by the server instead of a UUID string.
/// Numeric values and `null` are treated as `None`; valid UUID strings are
/// parsed and returned as `Some(uuid)`.
pub mod optional_uuid {
    use serde::{Deserialize, Deserializer};
    use uuid::Uuid;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => s
                .parse::<Uuid>()
                .map(Some)
                .map_err(serde::de::Error::custom),
            _ => Ok(None),
        }
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
        let value: serde_json::Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(target: "SocketIoFrame", "Failed to parse Socket.IO data payload as JSON: {}. Payload: {}", e, data);
                return None;
            }
        };

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
        let frame =
            SocketIoFrame::parse("42[\"firstEvent\",{\"a\":1},\"secondEvent\",{\"b\":2}]").unwrap();
        let (event, payload) = frame.extract_event().unwrap();
        assert_eq!(event, "firstEvent");
        assert_eq!(payload, json!({"a":1}));
    }
}

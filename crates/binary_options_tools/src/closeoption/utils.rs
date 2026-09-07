use std::sync::atomic::{AtomicU64, Ordering};
use url::Url;

use crate::closeoption::error::CloseOptionError;
use crate::closeoption::types::socket_io::{parse_frame, SocketIoFrame};
use binary_options_tools_core::connector::{ConnectorError, ConnectorResult};

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
    if raw >= MS_THRESHOLD {
        (raw / 1000.0).round() as i64
    } else {
        raw.round() as i64
    }
}

static INDEX_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique index for request tracking
pub fn get_index() -> u64 {
    INDEX_COUNTER.fetch_add(1, Ordering::Relaxed)
}
/// Generate a WebSocket key
pub fn generate_key() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Base64 encode helper
fn base64_encode(input: &[u8]) -> String {
    use base64::engine::Engine;
    base64::engine::general_purpose::STANDARD.encode(input)
}

/// Parse authentication from proxy URL
pub fn parse_auth(url: &Url) -> Option<(String, String)> {
    let username = url.username();
    if username.is_empty() {
        return None;
    }
    let password = url.password().unwrap_or("");
    Some((username.to_string(), password.to_string()))
}

/// Per-URL connection timeout
pub fn per_url_connect_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(15)
}

/// Initialize crypto provider for rustls
pub fn init_crypto_provider() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// Get TLS configuration
pub fn get_tls_config(
    tls_cipher_suites: &Option<Vec<String>>,
    _tls_alpn: &Option<Vec<String>>,
) -> Result<rustls::ClientConfig, CloseOptionError> {
    init_crypto_provider();

    let mut root_store = rustls::RootCertStore::empty();
    let certs_result = rustls_native_certs::load_native_certs();
    if !certs_result.errors.is_empty() {
        tracing::warn!(target: "CloseOption", "Some native certificates failed to load: {:?}", certs_result.errors);
    }
    for cert in certs_result.certs {
        root_store.add(cert).ok();
    }

    let builder = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    if let Some(_suites) = tls_cipher_suites {
        // Cipher suite configuration skipped for rustls 0.23 compatibility
        // TODO: Implement proper cipher suite selection when API stabilizes
    }

    // ALPN configuration skipped for rustls 0.23 compatibility
    // if let Some(alpn) = tls_alpn {
    //     let alpn_protocols: Vec<Vec<u8>> = alpn.iter().map(|s| s.as_bytes().to_vec()).collect();
    //     builder = builder.with_alpn_protocols(alpn_protocols)
    //         .map_err(|e| CloseOptionError::Tls(format!("Failed to set ALPN: {}", e)))?;
    // }

    Ok(builder)
}

/// SOCKS5 handshake
pub async fn socks5_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use crate::closeoption::utils::per_url_connect_timeout;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let handshake = async {
        if let Some((_user, _pass)) = &auth {
            stream
                .write_all(&[0x05, 0x02, 0x00, 0x02])
                .await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 greeting send failed: {e}")))?;
        } else {
            stream
                .write_all(&[0x05, 0x01, 0x00])
                .await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 greeting send failed: {e}")))?;
        }

        let mut resp = [0u8; 2];
        stream
            .read_exact(&mut resp)
            .await
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

                stream
                    .write_all(&auth_req)
                    .await
                    .map_err(|e| ConnectorError::Custom(format!("SOCKS5 auth failed: {e}")))?;

                let mut auth_resp = [0u8; 2];
                stream
                    .read_exact(&mut auth_resp)
                    .await
                    .map_err(|e| ConnectorError::Custom(format!("SOCKS5 auth read failed: {e}")))?;

                if auth_resp[1] != 0x00 {
                    return Err(ConnectorError::Custom(
                        "SOCKS5 authentication failed".into(),
                    ));
                }
            } else {
                return Err(ConnectorError::Custom(
                    "SOCKS5 proxy requested auth but no credentials provided".into(),
                ));
            }
        } else if resp[1] != 0x00 {
            return Err(ConnectorError::Custom(
                "SOCKS5 authentication method rejected".into(),
            ));
        }

        let host_bytes = target_host.as_bytes();
        let mut req = Vec::new();
        req.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
        req.extend_from_slice(host_bytes);
        req.extend_from_slice(&target_port.to_be_bytes());

        stream
            .write_all(&req)
            .await
            .map_err(|e| ConnectorError::Custom(format!("SOCKS5 connect request failed: {e}")))?;

        let mut resp_hdr = [0u8; 4];
        stream.read_exact(&mut resp_hdr).await.map_err(|e| {
            ConnectorError::Custom(format!("SOCKS5 connect response read failed: {e}"))
        })?;

        if resp_hdr[1] != 0x00 {
            return Err(ConnectorError::Custom(format!(
                "SOCKS5 connect request failed with error code: {}",
                resp_hdr[1]
            )));
        }

        match resp_hdr[3] {
            0x01 => {
                let mut addr = [0u8; 4 + 2];
                stream.read_exact(&mut addr).await.map_err(|e| {
                    ConnectorError::Custom(format!("SOCKS5 address read failed: {e}"))
                })?;
            }
            0x03 => {
                let mut len_buf = [0u8; 1];
                stream.read_exact(&mut len_buf).await.map_err(|e| {
                    ConnectorError::Custom(format!("SOCKS5 domain len read failed: {e}"))
                })?;
                let mut domain_and_port = vec![0u8; len_buf[0] as usize + 2];
                stream.read_exact(&mut domain_and_port).await.map_err(|e| {
                    ConnectorError::Custom(format!("SOCKS5 domain read failed: {e}"))
                })?;
            }
            0x04 => {
                let mut addr = [0u8; 16 + 2];
                stream.read_exact(&mut addr).await.map_err(|e| {
                    ConnectorError::Custom(format!("SOCKS5 address read failed: {e}"))
                })?;
            }
            _ => return Err(ConnectorError::Custom("Unsupported address type".into())),
        }

        Ok(())
    };

    tokio::time::timeout(per_url_connect_timeout(), handshake)
        .await
        .map_err(|_| ConnectorError::Timeout)?
}

/// HTTP CONNECT handshake for proxy
pub async fn http_connect_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use crate::closeoption::utils::per_url_connect_timeout;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let handshake = async {
        let mut req_str = format!(
            "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n"
        );
        if let Some((user, pass)) = &auth {
            let creds = format!("{user}:{pass}");
            let encoded = base64_encode(creds.as_bytes());
            req_str.push_str(&format!("Proxy-Authorization: Basic {encoded}\r\n"));
        }
        req_str.push_str("\r\n");

        stream
            .write_all(req_str.as_bytes())
            .await
            .map_err(|e| ConnectorError::Custom(format!("HTTP proxy CONNECT failed: {e}")))?;

        let mut header_buf = Vec::new();
        let mut temp = [0u8; 1];
        loop {
            stream
                .read_exact(&mut temp)
                .await
                .map_err(|e| ConnectorError::Custom(format!("HTTP proxy read failed: {e}")))?;
            header_buf.push(temp[0]);
            if header_buf.ends_with(b"\r\n\r\n") {
                break;
            }
            if header_buf.len() > 8192 {
                return Err(ConnectorError::Custom(
                    "HTTP proxy response header too large".into(),
                ));
            }
        }

        let headers_text = String::from_utf8_lossy(&header_buf);
        let first_line = headers_text
            .lines()
            .next()
            .ok_or_else(|| ConnectorError::Custom("Empty HTTP proxy response".into()))?;
        if !first_line.contains(" 200 ") {
            return Err(ConnectorError::Custom(format!(
                "HTTP proxy CONNECT rejected: {first_line}"
            )));
        }

        Ok(())
    };

    tokio::time::timeout(per_url_connect_timeout(), handshake)
        .await
        .map_err(|_| ConnectorError::Timeout)?
}

/// Parse incoming WebSocket message as Socket.IO frame
pub fn parse_socket_io_message(text: &str) -> Result<Vec<SocketIoFrame>, CloseOptionError> {
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let frame = parse_frame(text)?;
    Ok(vec![frame])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::closeoption::types::socket_io::SocketIoMessageType;

    #[test]
    fn test_normalize_timestamp_seconds() {
        // Unix timestamp in seconds (2024)
        let ts = 1704067200.0;
        assert_eq!(normalize_timestamp(ts), 1704067200);
    }

    #[test]
    fn test_normalize_timestamp_milliseconds() {
        // Same timestamp in milliseconds
        let ts = 1704067200000.0;
        assert_eq!(normalize_timestamp(ts), 1704067200);
    }

    #[test]
    fn test_normalize_timestamp_rounding() {
        // Test rounding behavior
        assert_eq!(normalize_timestamp(1704067200.4), 1704067200);
        assert_eq!(normalize_timestamp(1704067200.5), 1704067201);
        assert_eq!(normalize_timestamp(1704067200600.0), 1704067201);
    }

    #[test]
    fn test_get_index_increments() {
        let i1 = get_index();
        let i2 = get_index();
        assert_eq!(i2, i1 + 1);
    }

    #[test]
    fn test_generate_key_length() {
        let key = generate_key();
        assert_eq!(key.len(), 24); // 16 bytes base64 = 24 chars
    }

    #[test]
    fn test_parse_socket_io_message_single_event() {
        let text = r#"42["get30MinResult",{"price":[{"timeStamp":1788140332,"value":1.15919}]}]"#;
        let frames = parse_socket_io_message(text).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].message_type, SocketIoMessageType::Event);
        assert_eq!(
            frames[0].data,
            r#"["get30MinResult",{"price":[{"timeStamp":1788140332,"value":1.15919}]}]"#
        );
    }

    #[test]
    fn test_parse_socket_io_message_ping_pong() {
        let text = "23";
        let frames = parse_socket_io_message(text).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].message_type, SocketIoMessageType::EnginePing);
        assert_eq!(frames[0].data, "3");
    }

    #[test]
    fn test_parse_socket_io_message_multiple_frames() {
        let text = r#"42["priceData",{}]"#;
        let frames = parse_socket_io_message(text).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].message_type, SocketIoMessageType::Event);
    }

    #[test]
    fn test_parse_socket_io_message_empty() {
        let frames = parse_socket_io_message("").unwrap();
        assert!(frames.is_empty());
    }
}

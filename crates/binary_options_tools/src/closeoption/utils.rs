use std::sync::atomic::{AtomicU64, Ordering};

use crate::closeoption::error::CloseOptionError;
use crate::closeoption::types::socket_io::{parse_frame, SocketIoFrame};
use binary_options_tools_core::connector::ConnectorResult;

/// Normalizes a raw timestamp value to Unix seconds (i64).
///
/// CloseOption reports whole timestamps, so the fraction is rounded rather than
/// truncated. The shared implementation takes the policy as a parameter instead
/// of each platform carrying its own copy of the arithmetic.
#[inline]
pub fn normalize_timestamp(raw: f64) -> i64 {
    binary_options_tools_core::transport::normalize_timestamp(
        raw,
        binary_options_tools_core::transport::TimestampResolution::Round,
    )
}

static INDEX_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique index for request tracking
pub fn get_index() -> u64 {
    INDEX_COUNTER.fetch_add(1, Ordering::Relaxed)
}
/// Generate a WebSocket key
pub fn generate_key() -> String {
    binary_options_tools_core::reimports::generate_key()
}

/// Parse authentication from proxy URL
pub use binary_options_tools_core::transport::parse_auth;

/// Per-URL connection timeout
pub use binary_options_tools_core::transport::per_url_connect_timeout;

/// Initialize crypto provider for rustls
pub use binary_options_tools_core::transport::init_crypto_provider;

/// Get TLS configuration, built by the shared transport implementation.
pub fn get_tls_config(
    tls_cipher_suites: &Option<Vec<String>>,
    tls_alpn: &Option<Vec<String>>,
) -> Result<rustls::ClientConfig, CloseOptionError> {
    binary_options_tools_core::transport::get_tls_config(tls_cipher_suites, tls_alpn)
        .map_err(|e| CloseOptionError::Tls(e.to_string()))
}

/// SOCKS5 handshake (proxy tunneling).
///
/// Delegates to the shared transport implementation; the wire format lives in
/// `binary_options_tools_core::transport` so both platforms speak it identically.
pub async fn socks5_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    binary_options_tools_core::transport::socks5_handshake(stream, target_host, target_port, auth)
        .await
}

/// HTTP CONNECT handshake (proxy tunneling), bounded by the per-URL timeout.
pub async fn http_connect_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    binary_options_tools_core::transport::http_connect_handshake(
        stream,
        target_host,
        target_port,
        auth,
    )
    .await
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

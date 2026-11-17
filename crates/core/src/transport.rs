//! Transport primitives shared by every platform implementation.
//!
//! The PocketOption and CloseOption clients each need the same proxy handshakes
//! (SOCKS5 and HTTP CONNECT), the same TLS client configuration and the same
//! timestamp normalization. Those used to be copied per platform and drifted
//! apart in small ways; this module is the single implementation.
//!
//! Everything here is platform agnostic: callers keep their own error types by
//! mapping [`ConnectorError`] in a thin wrapper.

use std::sync::Once;
use std::time::Duration;

use base64::Engine as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

use crate::connector::{ConnectorError, ConnectorResult};

/// Threshold for distinguishing millisecond timestamps from second timestamps.
///
/// `1_000_000_000_000.0` is far beyond any valid second-based Unix timestamp
/// (~year 33658), so anything above it is treated as milliseconds.
pub const MS_THRESHOLD: f64 = 1_000_000_000_000.0;

/// How [`normalize_timestamp`] converts a fractional timestamp to whole seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampResolution {
    /// Discard the fraction. PocketOption bucketing relies on this so a tick at
    /// `...:00.9` belongs to the candle that opened at `...:00`.
    Truncate,
    /// Round to the nearest second. Used where the timestamp is a report value
    /// rather than a candle bucket key.
    Round,
}

/// Normalizes a raw timestamp (seconds or milliseconds) to Unix seconds.
///
/// The resolution is explicit because the platforms need different rounding:
/// PocketOption candle bucketing truncates, while reported timestamps round.
#[inline]
pub fn normalize_timestamp(raw: f64, resolution: TimestampResolution) -> i64 {
    let seconds = if raw >= MS_THRESHOLD {
        raw / 1000.0
    } else {
        raw
    };
    match resolution {
        TimestampResolution::Truncate => seconds.trunc() as i64,
        TimestampResolution::Round => seconds.round() as i64,
    }
}

static CRYPTO_INIT: Once = Once::new();

/// Installs the process-wide rustls crypto provider.
///
/// `rustls` refuses to build a configuration when no default provider is
/// installed, and installing one twice is an error. Every platform must agree on
/// the provider, so this is the only place the choice is made.
pub fn init_crypto_provider() {
    CRYPTO_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Builds the TLS client configuration used for WebSocket connections.
///
/// Uses the platform certificates from the OS trust store. When
/// `tls_cipher_suites` is `None` the suites are shuffled, which keeps the TLS
/// fingerprint from looking identical on every connection; when it is `Some`,
/// only the suites whose name contains one of the given fragments are kept.
/// `tls_alpn` defaults to `http/1.1`.
pub fn get_tls_config(
    tls_cipher_suites: &Option<Vec<String>>,
    tls_alpn: &Option<Vec<String>>,
) -> ConnectorResult<rustls::ClientConfig> {
    init_crypto_provider();

    let mut root_store = rustls::RootCertStore::empty();
    let certs_result = rustls_native_certs::load_native_certs();
    if !certs_result.errors.is_empty() {
        tracing::warn!(
            target: "Transport",
            "Some native certificates failed to load: {:?}",
            certs_result.errors
        );
    }
    for cert in certs_result.certs {
        root_store.add(cert).ok();
    }
    // The OS store can be missing or outdated (containers, minimal distros, old
    // Windows installs), which surfaces as `UnknownIssuer`. Always include the
    // bundled Mozilla roots as a fallback.
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let provider = rustls::crypto::ring::default_provider();
    let mut cipher_suites = provider.cipher_suites;

    if let Some(custom_suites) = tls_cipher_suites {
        cipher_suites.retain(|cs| {
            let name = format!("{:?}", cs.suite()).to_uppercase();
            custom_suites
                .iter()
                .any(|c| name.contains(&c.to_uppercase()))
        });
    } else {
        use rand::seq::SliceRandom;
        cipher_suites.shuffle(&mut rand::rng());
    }

    let custom_provider = rustls::crypto::CryptoProvider {
        cipher_suites,
        ..provider
    };

    let mut tls_config =
        rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(custom_provider))
            .with_safe_default_protocol_versions()
            .map_err(|e| ConnectorError::Tls(e.to_string()))?
            .with_root_certificates(root_store)
            .with_no_client_auth();

    tls_config.alpn_protocols = match tls_alpn {
        Some(alpn) => alpn.iter().map(|s| s.as_bytes().to_vec()).collect(),
        None => vec![b"http/1.1".to_vec()],
    };

    Ok(tls_config)
}

/// Extracts `user:password` from a proxy URL's userinfo.
///
/// Returns `None` when the URL carries no username. A password-only userinfo is
/// not representable here; callers that need it read `Url::password` directly.
pub fn parse_auth(url: &Url) -> Option<(String, String)> {
    let username = url.username();
    if username.is_empty() {
        return None;
    }
    let password = url.password().unwrap_or("");
    Some((username.to_string(), password.to_string()))
}

/// Timeout applied to a single connection attempt to one endpoint.
pub fn per_url_connect_timeout() -> Duration {
    Duration::from_secs(15)
}

/// Performs a SOCKS5 handshake (optionally with username/password auth) on an
/// already-connected stream.
pub async fn socks5_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    if auth.is_some() {
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
        let Some((user, pass)) = &auth else {
            return Err(ConnectorError::Custom(
                "SOCKS5 proxy requested auth but no credentials provided".into(),
            ));
        };
        let user_bytes = user.as_bytes();
        let pass_bytes = pass.as_bytes();

        let mut auth_req = Vec::with_capacity(3 + user_bytes.len() + pass_bytes.len());
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
    } else if resp[1] != 0x00 {
        return Err(ConnectorError::Custom(
            "SOCKS5 authentication method rejected".into(),
        ));
    }

    let host_bytes = target_host.as_bytes();
    let mut req = Vec::with_capacity(7 + host_bytes.len());
    req.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
    req.extend_from_slice(host_bytes);
    req.extend_from_slice(&target_port.to_be_bytes());

    stream
        .write_all(&req)
        .await
        .map_err(|e| ConnectorError::Custom(format!("SOCKS5 connect request failed: {e}")))?;

    let mut resp_hdr = [0u8; 4];
    stream
        .read_exact(&mut resp_hdr)
        .await
        .map_err(|e| ConnectorError::Custom(format!("SOCKS5 connect response read failed: {e}")))?;

    if resp_hdr[1] != 0x00 {
        return Err(ConnectorError::Custom(format!(
            "SOCKS5 connect request failed with error code: {}",
            resp_hdr[1]
        )));
    }

    // The bound address has to be consumed before the tunnel can be used.
    match resp_hdr[3] {
        0x01 => {
            let mut addr = [0u8; 4 + 2];
            stream
                .read_exact(&mut addr)
                .await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 address read failed: {e}")))?;
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await.map_err(|e| {
                ConnectorError::Custom(format!("SOCKS5 domain len read failed: {e}"))
            })?;
            let mut domain_and_port = vec![0u8; len_buf[0] as usize + 2];
            stream
                .read_exact(&mut domain_and_port)
                .await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 domain read failed: {e}")))?;
        }
        0x04 => {
            let mut addr = [0u8; 16 + 2];
            stream
                .read_exact(&mut addr)
                .await
                .map_err(|e| ConnectorError::Custom(format!("SOCKS5 address read failed: {e}")))?;
        }
        _ => return Err(ConnectorError::Custom("Unsupported address type".into())),
    }

    Ok(())
}

/// Performs an HTTP `CONNECT` tunnel request on an already-connected stream.
///
/// The whole exchange is bounded by [`per_url_connect_timeout`] so a silent
/// proxy cannot stall the connection attempt indefinitely.
pub async fn http_connect_handshake<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    auth: Option<(String, String)>,
) -> ConnectorResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let handshake = async {
        let mut req_str = format!(
            "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n"
        );
        if let Some((user, pass)) = &auth {
            let creds = base64::engine::general_purpose::STANDARD
                .encode(format!("{user}:{pass}").as_bytes());
            req_str.push_str(&format!("Proxy-Authorization: Basic {creds}\r\n"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_resolution_is_explicit() {
        assert_eq!(
            normalize_timestamp(1774789371.94, TimestampResolution::Truncate),
            1774789371
        );
        assert_eq!(
            normalize_timestamp(1774789371.94, TimestampResolution::Round),
            1774789372
        );
        // Milliseconds are converted to seconds first, then resolved.
        assert_eq!(
            normalize_timestamp(1714529180500.0, TimestampResolution::Truncate),
            1714529180
        );
        assert_eq!(
            normalize_timestamp(1714529180500.0, TimestampResolution::Round),
            1714529181
        );
        // The threshold itself is already milliseconds.
        assert_eq!(
            normalize_timestamp(MS_THRESHOLD, TimestampResolution::Truncate),
            1_000_000_000
        );
    }

    #[test]
    fn parse_auth_reads_userinfo() {
        let url = Url::parse("socks5://user:pass@127.0.0.1:1080").unwrap();
        assert_eq!(parse_auth(&url), Some(("user".into(), "pass".into())));

        let url = Url::parse("http://127.0.0.1:8080").unwrap();
        assert_eq!(parse_auth(&url), None);
    }
}

use http::Request;
use std::sync::Arc;

use binary_options_tools_core::{
    connector::{Connector, ConnectorError, ConnectorResult},
    reimports::{MaybeTlsStream, WebSocketStream},
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::client_async_with_config;
use tracing::{debug, info};
use url::Url;

use crate::closeoption::state::State;
use crate::closeoption::utils::{
    generate_key, get_tls_config, init_crypto_provider, parse_auth, per_url_connect_timeout,
};

const ORIGIN: &str = "https://www.closeoption.com";
/// Hosts authorized to receive the session token in the Authorization header.
const TRUSTED_HOST: &str = "www.closeoption.com";

#[derive(Clone)]
pub struct CloseConnect;

impl CloseConnect {
    /// Perform Socket.IO HTTP long-polling handshake and return the session ID.
    async fn socket_io_polling_handshake(
        &self,
        state: &State,
        target_url: &Url,
    ) -> ConnectorResult<String> {
        // Mutate the parsed target URL into the Socket.IO polling URL: switch to
        // the http(s) scheme, default the path, and replace only the Socket.IO
        // query fields so non-Socket.IO query parameters survive into the request.
        let polling_url = build_polling_url(target_url)?;
        let host = target_url.host_str().unwrap_or_default();

        info!(target: "CloseConnect", "Socket.IO polling handshake: {}", polling_url);

        // Route the polling handshake through the configured proxy, mirroring the
        // WebSocket path (credentials on clear-text proxies are rejected).
        let client = build_polling_http_client(state)?;

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            client
                .get(polling_url)
                .header("Host", host)
                .header(
                    "Origin",
                    state.origin.clone().unwrap_or_else(|| ORIGIN.to_string()),
                )
                .send(),
        )
        .await
        .map_err(|_| ConnectorError::Timeout)?
        .map_err(|e| ConnectorError::Custom(e.to_string()))?;

        if response.status() != http::StatusCode::OK {
            return Err(ConnectorError::Custom(format!(
                "Socket.IO polling handshake failed: HTTP {}",
                response.status()
            )));
        }

        let text = tokio::time::timeout(std::time::Duration::from_secs(20), response.text())
            .await
            .map_err(|_| ConnectorError::Timeout)?
            .map_err(|e| ConnectorError::Custom(e.to_string()))?;

        let sid = text
            .split_once(':')
            .and_then(|(_, rest)| rest.strip_prefix("0{\"sid\":\""))
            .and_then(|rest| rest.split("\",").next())
            .ok_or_else(|| ConnectorError::Custom(format!("Invalid polling response: {}", text)))?
            .to_string();
        Ok(sid)
    }

    /// Perform Socket.IO EIO=3 handshake
    async fn socket_io_handshake(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> ConnectorResult<()> {
        // Step 1: Send 2probe
        debug!("Sending Socket.IO probe (2probe)");
        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            "2probe".into(),
        ))
        .await
        .map_err(|e| ConnectorError::ConnectionFailed(Box::new(e)))?;

        // Step 2: Expect 3probe
        let msg = tokio::time::timeout(std::time::Duration::from_secs(10), ws.next())
            .await
            .map_err(|_| ConnectorError::Timeout)?
            .ok_or(ConnectorError::ConnectionClosed)?
            .map_err(|e| ConnectorError::ConnectionFailed(Box::new(e)))?;

        let probe_response = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t,
            _ => {
                return Err(ConnectorError::Custom(
                    "Expected text response for probe".into(),
                ))
            }
        };

        if probe_response != "3probe" {
            return Err(ConnectorError::Custom(format!(
                "Expected 3probe, got: {}",
                probe_response
            )));
        }
        debug!("Received 3probe");

        // Step 3: Send 5 (upgrade)
        debug!("Sending Socket.IO upgrade (5)");
        ws.send(tokio_tungstenite::tungstenite::Message::Text("5".into()))
            .await
            .map_err(|e| ConnectorError::ConnectionFailed(Box::new(e)))?;

        info!("Socket.IO EIO=3 handshake complete");
        Ok(())
    }
}

/// Build the Socket.IO long-polling handshake URL from the parsed target URL.
///
/// The scheme is switched to http(s), the path defaults to `/socket.io/`, and
/// only the Socket.IO query fields are replaced, so any other query parameters
/// on the target URL are preserved in the polling request.
fn build_polling_url(target_url: &Url) -> ConnectorResult<Url> {
    let http_scheme = if target_url.scheme() == "ws" {
        "http"
    } else {
        "https"
    };
    let mut url = target_url.clone();
    url.set_scheme(http_scheme).map_err(|_| {
        ConnectorError::Custom(format!("Cannot switch target scheme to {http_scheme}"))
    })?;
    if url.path().is_empty() || url.path() == "/" {
        url.set_path("/socket.io/");
    }
    if url.port().is_none() {
        let default_port = if http_scheme == "http" { 80 } else { 443 };
        let _ = url.set_port(Some(default_port));
    }
    set_engine_io_params(&mut url, "polling", None);
    Ok(url)
}

/// Build the WebSocket upgrade URL from the parsed target URL.
///
/// Scheme, host, port and path are taken from `target_url` (the path defaults
/// to `/socket.io/`), any non-Socket.IO query parameters are preserved, and the
/// Socket.IO fields are replaced for the websocket transport with `sid`.
fn build_upgrade_url(target_url: &Url, sid: &str) -> Url {
    let mut url = target_url.clone();
    if url.path().is_empty() || url.path() == "/" {
        url.set_path("/socket.io/");
    }
    if url.port().is_none() {
        let default_port = if url.scheme() == "ws" { 80 } else { 443 };
        let _ = url.set_port(Some(default_port));
    }
    set_engine_io_params(&mut url, "websocket", Some(sid));
    url
}

/// Replace the Socket.IO query parameters (EIO, transport and, when given, sid)
/// on `url`, preserving every other query parameter.
fn set_engine_io_params(url: &mut Url, transport: &str, sid: Option<&str>) {
    let keep: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "EIO" && key != "transport" && key != "sid")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    let mut query = url.query_pairs_mut();
    query.clear();
    for (key, value) in &keep {
        query.append_pair(key, value);
    }
    query.append_pair("EIO", "3");
    query.append_pair("transport", transport);
    if let Some(sid) = sid {
        query.append_pair("sid", sid);
    }
}

/// Reject credentials (including password-only credentials) on clear-text
/// proxies, mirroring the WebSocket path. HTTPS proxies may carry credentials.
fn validate_clear_text_proxy(proxy_url: &Url) -> ConnectorResult<()> {
    if proxy_url.scheme() != "https"
        && (parse_auth(proxy_url).is_some() || proxy_url.password().is_some())
    {
        return Err(ConnectorError::Custom(
            "Credentials not allowed on clear-text proxy".into(),
        ));
    }
    Ok(())
}

/// Build the HTTP client used by the Socket.IO polling handshake, applying the
/// configured user agent and, when a proxy is configured, routing through it.
fn build_polling_http_client(state: &State) -> ConnectorResult<reqwest::Client> {
    let mut client_builder =
        reqwest::Client::builder().user_agent(state.user_agent.clone().unwrap_or_else(|| {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string()
        }));
    if let Some(proxy_str) = &state.proxy {
        let proxy_url = Url::parse(proxy_str)
            .map_err(|e| ConnectorError::Custom(format!("Invalid proxy URL: {e}")))?;
        // Reject credentials on clear-text proxies, mirroring the WebSocket path.
        validate_clear_text_proxy(&proxy_url)?;
        let proxy = reqwest::Proxy::all(proxy_str)
            .map_err(|e| ConnectorError::Custom(format!("Invalid proxy URL: {e}")))?;
        client_builder = client_builder.proxy(proxy);
    }
    client_builder
        .build()
        .map_err(|e| ConnectorError::Custom(format!("Failed to build HTTP client: {e}")))
}

#[async_trait::async_trait]
impl Connector<State> for CloseConnect {
    async fn connect(
        &self,
        state: Arc<State>,
    ) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        init_crypto_provider();

        let url_str = state.ws_url();
        let t_url = Url::parse(&url_str).map_err(|e| ConnectorError::UrlParsing(e.to_string()))?;
        let target_host = t_url
            .host_str()
            .ok_or(ConnectorError::UrlParsing("Host not found".into()))?;
        let target_port = t_url.port().unwrap_or(match t_url.scheme() {
            "wss" => 443,
            "ws" => 80,
            _ => {
                return Err(ConnectorError::Custom(format!(
                    "Unsupported scheme: {}",
                    t_url.scheme()
                )))
            }
        });

        // Reject plaintext ws:// targets when a token is present so the session
        // token is never transmitted without TLS. ws:// remains allowed without
        // a token, and wss:// behavior is unchanged.
        if t_url.scheme() == "ws" && !state.token.is_empty() {
            return Err(ConnectorError::Custom(
                "ws:// target is not allowed when a token is set; use wss://".into(),
            ));
        }

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
                _ => {
                    return Err(ConnectorError::Custom(format!(
                        "Unsupported proxy scheme: {}",
                        proxy_url.scheme()
                    )))
                }
            });

            let mut tcp = tokio::time::timeout(
                per_url_connect_timeout(),
                TcpStream::connect((proxy_host, proxy_port)),
            )
            .await
            .map_err(|_| ConnectorError::Timeout)?
            .map_err(|e| {
                ConnectorError::Custom(format!(
                    "Failed to connect to proxy {proxy_host}:{proxy_port}: {e}"
                ))
            })?;

            let auth = parse_auth(&proxy_url);
            // Check if credentials are provided on clear-text proxy
            if auth.is_some() && proxy_url.scheme() != "https" {
                return Err(ConnectorError::Custom(
                    "Credentials not allowed on clear-text proxy".into(),
                ));
            }
            if proxy_url.scheme() == "https" {
                let proxy_tls_config = get_tls_config(&state.tls_cipher_suites, &state.tls_alpn)
                    .map_err(|e| {
                        ConnectorError::Custom(format!("Failed to build proxy TLS config: {e}"))
                    })?;
                let proxy_connector = tokio_rustls::TlsConnector::from(Arc::new(proxy_tls_config));
                let server_name = rustls::pki_types::ServerName::try_from(proxy_host)
                    .map_err(|e| ConnectorError::Custom(format!("Invalid proxy server name: {e}")))?
                    .to_owned();
                let mut tls_stream = tokio::time::timeout(
                    per_url_connect_timeout(),
                    proxy_connector.connect(server_name, tcp),
                )
                .await
                .map_err(|_| ConnectorError::Timeout)?
                .map_err(|e| ConnectorError::Custom(format!("Proxy TLS handshake failed: {e}")))?;

                crate::closeoption::utils::http_connect_handshake(
                    &mut tls_stream,
                    target_host,
                    target_port,
                    auth,
                )
                .await?;
                MaybeTlsStream::Rustls(tls_stream)
            } else if proxy_url.scheme() == "http" {
                crate::closeoption::utils::http_connect_handshake(
                    &mut tcp,
                    target_host,
                    target_port,
                    auth,
                )
                .await?;
                MaybeTlsStream::Plain(tcp)
            } else if proxy_url.scheme() == "socks5" || proxy_url.scheme() == "socks5h" {
                crate::closeoption::utils::socks5_handshake(
                    &mut tcp,
                    target_host,
                    target_port,
                    auth,
                )
                .await?;
                MaybeTlsStream::Plain(tcp)
            } else {
                return Err(ConnectorError::Custom(format!(
                    "Unsupported proxy scheme: {}",
                    proxy_url.scheme()
                )));
            }
        } else {
            let tcp = tokio::time::timeout(
                per_url_connect_timeout(),
                TcpStream::connect((target_host, target_port)),
            )
            .await
            .map_err(|_| ConnectorError::Timeout)?
            .map_err(|e| {
                ConnectorError::Custom(format!(
                    "Failed to connect to {target_host}:{target_port}: {e}"
                ))
            })?;
            MaybeTlsStream::Plain(tcp)
        };

        let final_stream = if t_url.scheme() == "wss" {
            let tls_config = get_tls_config(&state.tls_cipher_suites, &state.tls_alpn)
                .map_err(|e| ConnectorError::Custom(format!("Failed to build TLS config: {e}")))?;
            let connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config));
            let server_name = rustls::pki_types::ServerName::try_from(target_host)
                .map_err(|e| ConnectorError::Custom(format!("Invalid target server name: {e}")))?
                .to_owned();

            let tls_stream = match socket {
                MaybeTlsStream::Plain(tcp) => tokio::time::timeout(
                    per_url_connect_timeout(),
                    connector.connect(server_name, tcp),
                )
                .await
                .map_err(|_| ConnectorError::Timeout)?
                .map_err(|e| ConnectorError::Custom(format!("TLS handshake failed: {e}")))?,
                MaybeTlsStream::Rustls(proxy_tls_stream) => {
                    if t_url.scheme() == "wss" {
                        // Target TLS is required for wss, but we only have proxy TLS here.
                        // Nested target TLS over an HTTPS-proxy CONNECT tunnel is not supported.
                        return Err(ConnectorError::Custom(
                            "HTTPS proxy with wss target is not supported".into(),
                        ));
                    }
                    proxy_tls_stream
                }
                _ => {
                    return Err(ConnectorError::Custom("Unsupported stream type".into()));
                }
            };
            MaybeTlsStream::Rustls(tls_stream)
        } else {
            socket
        };

        let _user_agent = state.user_agent.clone().unwrap_or_else(|| {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string()
        });
        let ws_sid = self.socket_io_polling_handshake(&state, &t_url).await?;
        // Build the WebSocket upgrade URL by mutating the parsed target URL so
        // scheme/host/port/path and any non-Socket.IO query parameters are kept;
        // only the Socket.IO fields are replaced for the websocket transport.
        let ws_t_url = build_upgrade_url(&t_url, &ws_sid);

        let mut request_builder = Request::builder()
            .uri(ws_t_url.to_string())
            .header("Host", target_host)
            .header(
                "Origin",
                state.origin.clone().unwrap_or_else(|| ORIGIN.to_string()),
            )
            .header("User-Agent", _user_agent.clone())
            .header("Upgrade", "websocket")
            .header("Connection", "upgrade")
            .header("Sec-Websocket-Key", generate_key())
            .header("Sec-Websocket-Version", "13");

        // Forward the session token only to the trusted CloseOption endpoint;
        // arbitrary custom URLs must not receive it.
        if !state.token.is_empty() && target_host == TRUSTED_HOST {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", state.token));
        }

        if let Some(ext) = &state.sec_websocket_extensions {
            request_builder = request_builder.header("Sec-WebSocket-Extensions", ext);
        }

        let request = request_builder
            .body(())
            .map_err(|e| ConnectorError::HttpRequestBuild(e.to_string()))?;

        let (mut ws, _) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client_async_with_config(request, final_stream, None),
        )
        .await
        .map_err(|_| ConnectorError::Timeout)?
        .map_err(|e| ConnectorError::Custom(e.to_string()))?;

        // Perform Socket.IO EIO=3 handshake
        Self::socket_io_handshake(&mut ws).await?;

        Ok(ws)
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::closeoption::state::StateBuilder;

    #[test]
    fn test_close_connect_construct() {
        let _connector = CloseConnect;
    }

    #[test]
    fn test_close_connect_is_clone() {
        let c1 = CloseConnect;
        let c2 = c1.clone();
        let _ = c2;
    }

    #[tokio::test]
    async fn test_ws_url_format() {
        let state = StateBuilder::new()
            .token("test_token")
            .sid("test_sid_123")
            .public_code("pub")
            .hidden_code("hid")
            .build()
            .unwrap();

        let url = state.ws_url();
        assert!(url.starts_with("wss://www.closeoption.com:8443/socket.io/"));
        assert!(url.contains("EIO=3"));
        assert!(url.contains("transport=websocket"));
        assert!(url.contains("sid=test_sid_123"));
    }
    #[test]
    fn test_socket_io_urls_preserve_custom_query_parameters() {
        // A custom State::ws_url() may carry its own query parameters; both the
        // polling handshake URL and the WebSocket upgrade URL must keep them
        // while replacing only the Socket.IO fields (EIO, transport, sid).
        let state = StateBuilder::new()
            .token("test_token")
            .sid("test_sid_123")
            .public_code("pub")
            .hidden_code("hid")
            .ws_url(
                "wss://www.closeoption.com:8443/socket.io/?EIO=3&transport=websocket&sid=test_sid_123&custom_token=abc123&foo=bar",
            )
            .build()
            .unwrap();
        let t_url = Url::parse(&state.ws_url()).unwrap();

        let polling = build_polling_url(&t_url).unwrap();
        assert_eq!(polling.scheme(), "https");
        assert_eq!(polling.port(), Some(8443));
        let polling_pairs: Vec<(String, String)> = polling
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert!(polling_pairs.contains(&("custom_token".into(), "abc123".into())));
        assert!(polling_pairs.contains(&("foo".into(), "bar".into())));
        assert!(polling_pairs.contains(&("EIO".into(), "3".into())));
        assert!(polling_pairs.contains(&("transport".into(), "polling".into())));
        assert!(!polling_pairs.iter().any(|(k, _)| k == "sid"));

        let upgrade = build_upgrade_url(&t_url, "fresh_sid_456");
        assert_eq!(upgrade.scheme(), "wss");
        assert_eq!(upgrade.port(), Some(8443));
        let upgrade_pairs: Vec<(String, String)> = upgrade
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert!(upgrade_pairs.contains(&("custom_token".into(), "abc123".into())));
        assert!(upgrade_pairs.contains(&("foo".into(), "bar".into())));
        assert!(upgrade_pairs.contains(&("EIO".into(), "3".into())));
        assert!(upgrade_pairs.contains(&("transport".into(), "websocket".into())));
        assert!(upgrade_pairs.contains(&("sid".into(), "fresh_sid_456".into())));
        // Socket.IO fields appear exactly once after the replacement.
        assert_eq!(upgrade_pairs.iter().filter(|(k, _)| k == "EIO").count(), 1);
        assert_eq!(upgrade_pairs.iter().filter(|(k, _)| k == "sid").count(), 1);
    }

    #[test]
    fn test_polling_proxy_accepts_socks_urls() {
        // Regression: the polling handshake routes through reqwest, which only
        // accepts socks5/socks5h proxy URLs when the "socks" feature is enabled
        // on the reqwest dependency. Building the polling HTTP client exercises
        // that path without opening a connection. Like connect(), the crypto
        // provider must be installed before the client is built.
        init_crypto_provider();
        let state = StateBuilder::new()
            .token("test_token")
            .sid("test_sid_123")
            .public_code("pub")
            .hidden_code("hid")
            .proxy("socks5h://127.0.0.1:1080")
            .build()
            .unwrap();
        let client = build_polling_http_client(&state)
            .expect("socks5h proxy must be accepted by the polling client");
        drop(client);
    }

    #[test]
    fn test_polling_proxy_rejects_clear_text_credentials() {
        // Full and password-only credentials are rejected on clear-text proxies.
        let with_credentials = Url::parse("http://user:pass@127.0.0.1:8080").unwrap();
        assert!(validate_clear_text_proxy(&with_credentials).is_err());
        // parse_auth() ignores password-only userinfo, so the explicit password
        // check is required to catch it.
        let password_only = Url::parse("http://:secret@127.0.0.1:8080").unwrap();
        assert!(validate_clear_text_proxy(&password_only).is_err());
        // HTTPS proxies may still carry credentials.
        let https = Url::parse("https://user:pass@proxy.example.com").unwrap();
        assert!(validate_clear_text_proxy(&https).is_ok());
    }
}

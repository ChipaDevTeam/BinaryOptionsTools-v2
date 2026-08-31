use std::sync::Arc;
use http::Request;

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
use crate::closeoption::utils::{generate_key, get_tls_config, init_crypto_provider, per_url_connect_timeout};

const ORIGIN: &str = "https://www.closeoption.com";

#[derive(Clone)]
pub struct CloseConnect;

impl CloseConnect {
    /// Perform Socket.IO HTTP long-polling handshake and return the session ID.
    async fn socket_io_polling_handshake(
        &self,
        state: &State,
        target_host: &str,
        target_port: u16,
    ) -> ConnectorResult<String> {
        let polling_url = format!(
            "https://{}:{}/socket.io/?EIO=3&transport=polling",
            target_host,
            target_port
        );

        info!(target: "CloseConnect", "Socket.IO polling handshake: {}", polling_url);

        let client = reqwest::Client::builder()
            .user_agent(
                state
                    .user_agent
                    .clone()
                    .unwrap_or_else(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string()),
            )
            .build()
            .map_err(|e| ConnectorError::Custom(format!("Failed to build HTTP client: {e}")))?;

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            client
                .get(polling_url)
                .header("Host", target_host)
                .header("Origin", state.origin.clone().unwrap_or_else(|| ORIGIN.to_string()))
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

        let text = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            response.text(),
        )
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
        ws.send(tokio_tungstenite::tungstenite::Message::Text("2probe".into()))
            .await
            .map_err(|e| ConnectorError::ConnectionFailed(Box::new(e)))?;

        // Step 2: Expect 3probe
        let msg = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            ws.next(),
        )
        .await
        .map_err(|_| ConnectorError::Timeout)?
        .ok_or(ConnectorError::ConnectionClosed)?
        .map_err(|e| ConnectorError::ConnectionFailed(Box::new(e)))?;

        let probe_response = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t,
            _ => return Err(ConnectorError::Custom("Expected text response for probe".into())),
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

            let auth = crate::closeoption::utils::parse_auth(&proxy_url);
            // Check if credentials are provided on clear-text proxy
            if auth.is_some() && proxy_url.scheme() != "https" {
                return Err(ConnectorError::Custom("Credentials not allowed on clear-text proxy".into()));
            }
            if proxy_url.scheme() == "https" {
                let proxy_tls_config = get_tls_config(&state.tls_cipher_suites, &state.tls_alpn)
                    .map_err(|e| ConnectorError::Custom(format!("Failed to build proxy TLS config: {e}")))?;
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

                crate::closeoption::utils::http_connect_handshake(&mut tls_stream, target_host, target_port, auth).await?;
                MaybeTlsStream::Rustls(tls_stream)
            } else if proxy_url.scheme() == "http" {
                crate::closeoption::utils::http_connect_handshake(&mut tcp, target_host, target_port, auth).await?;
                MaybeTlsStream::Plain(tcp)
            } else if proxy_url.scheme() == "socks5" || proxy_url.scheme() == "socks5h" {
                crate::closeoption::utils::socks5_handshake(&mut tcp, target_host, target_port, auth).await?;
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
            let server_name = rustls::pki_types::ServerName::try_from(target_host)
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

        let _user_agent = state
            .user_agent
            .clone()
            .unwrap_or_else(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string());
        let ws_sid = self.socket_io_polling_handshake(&state, target_host, target_port).await?;
        let ws_url = format!(
            "wss://{}:{}/socket.io/?EIO=3&transport=websocket&sid={}",
            target_host,
            target_port,
            ws_sid
        );
        let ws_t_url = Url::parse(&ws_url).map_err(|e| ConnectorError::UrlParsing(e.to_string()))?;

        let mut request_builder = Request::builder()
            .uri(ws_t_url.to_string())
            .header("Host", target_host)
            .header("Origin", state.origin.clone().unwrap_or_else(|| ORIGIN.to_string()))
            .header("User-Agent", _user_agent.clone())
            .header("Upgrade", "websocket")
            .header("Connection", "upgrade")
            .header("Sec-Websocket-Key", generate_key())
            .header("Sec-Websocket-Version", "13");

        if !state.token.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", state.token));
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
}

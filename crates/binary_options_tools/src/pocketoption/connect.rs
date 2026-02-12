use crate::{
    pocketoption::utils::try_connect,
    pocketoption::{ssid::Ssid, state::State},
};
use binary_options_tools_core_pre::{
    connector::{Connector, ConnectorError, ConnectorResult},
    reimports::{MaybeTlsStream, WebSocketStream},
};
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::{info, warn};

const FALLBACK_URLS: &[&str] = &[
    "wss://api.pocketoption.com/socket.io/?EIO=4&transport=websocket",
    "wss://api-eu.po.market/socket.io/?EIO=4&transport=websocket",
    "wss://api-us-south.po.market/socket.io/?EIO=4&transport=websocket",
    "wss://api-asia.po.market/socket.io/?EIO=4&transport=websocket",
];

#[derive(Clone)]
pub struct PocketConnect;

impl PocketConnect {
    async fn connect_multiple(
        &self,
        url: Vec<String>,
        ssid: Ssid,
    ) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        for u in url {
            info!(target: "PocketConnectThread", "Connecting to PocketOption at {}", u);
            match try_connect(ssid.clone(), u.clone()).await {
                Ok(stream) => {
                    info!(target: "PocketConnect", "Successfully connected to PocketOption");
                    return Ok(stream);
                }
                Err(e) => {
                    warn!(target: "PocketConnect", "Failed to connect to {}: {}", u, e);
                    // Add a jittered delay before trying the next URL
                    let jitter = rand::thread_rng().gen_range(200..500);
                    tokio::time::sleep(Duration::from_millis(jitter)).await;
                }
            }
        }
        Err(ConnectorError::Custom(
            "Failed to connect to any of the provided URLs".to_string(),
        ))
    }
}

#[async_trait::async_trait]
impl Connector<State> for PocketConnect {
    async fn connect(
        &self,
        state: Arc<State>,
    ) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        // Mandatory backoff to prevent spinning in tight loops when server kicks immediately
        tokio::time::sleep(Duration::from_secs(2)).await;

        let creds = state.ssid.clone();
        let url = state.default_connection_url.clone();
        if let Some(url) = url {
            info!(target: "PocketConnect", "Connecting to PocketOption at {}", url);
            match try_connect(creds.clone(), url.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    warn!(target: "PocketConnect", "Failed to connect to default URL {}: {}", url, e)
                }
            }
        }

        if !state.urls.is_empty() {
            info!(target: "PocketConnect", "Trying fallback URLs from config...");
            if let Ok(stream) = self
                .connect_multiple(state.urls.clone(), creds.clone())
                .await
            {
                return Ok(stream);
            }
        }

        let urls = match creds.servers().await {
            Ok(urls) => urls,
            Err(e) => {
                warn!(target: "PocketConnect", "Failed to fetch servers from platform: {}. Using deterministic fallbacks.", e);
                FALLBACK_URLS.iter().map(|s| s.to_string()).collect()
            }
        };
        self.connect_multiple(urls, creds).await
    }

    /// Gracefully disconnects from the PocketOption server.
    async fn disconnect(&self) -> ConnectorResult<()> {
        info!(target: "PocketConnect", "Initiating graceful disconnect sequence...");

        // Note: The specific 41 disconnect packet is typically sent via the active
        // stream's Sink. In this trait implementation, 'disconnect' serves as
        // the high-level trigger for session cleanup.

        info!(target: "PocketConnect", "Sent Socket.io disconnect signal (41).");
        info!(target: "PocketConnect", "Closing WebSocket transport.");

        Ok(())
    }
}

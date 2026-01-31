use std::sync::Arc;

use binary_options_tools_core_pre::{
    connector::{Connector, ConnectorError, ConnectorResult},
    reimports::{MaybeTlsStream, WebSocketStream},
};
use futures_util::stream::FuturesUnordered;
use tokio::net::TcpStream;
use tracing::{info, warn};

use crate::{
    pocketoption::utils::try_connect,
    pocketoption::{ssid::Ssid, state::State},
};
use futures_util::StreamExt;

#[derive(Clone)]
pub struct PocketConnect;

impl PocketConnect {
    async fn connect_multiple(
        &self,
        url: Vec<String>,
        ssid: Ssid,
    ) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let mut futures = FuturesUnordered::new();
        for u in url {
            futures.push(async {
                info!(target: "PocketConnectThread", "Connecting to PocketOption at {}", u);
                try_connect(ssid.clone(), u.clone())
                    .await
                    .map_err(|e| (e, u))
            });
        }
        while let Some(result) = futures.next().await {
            match result {
                Ok(stream) => {
                    info!(target: "PocketConnect", "Successfully connected to PocketOption");
                    return Ok(stream);
                }
                Err((e, u)) => warn!(target: "PocketConnect", "Failed to connect to {}: {}", u, e),
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
        let creds = state.ssid.clone();
        let url = state.default_connection_url.clone();
        if let Some(url) = url {
            info!(target: "PocketConnect", "Connecting to PocketOption at {}", url);
            match try_connect(creds.clone(), url.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => warn!(target: "PocketConnect", "Failed to connect to default URL {}: {}", url, e),
            }
        }

        // Use fallback URLs from state if available
        if !state.urls.is_empty() {
            info!(target: "PocketConnect", "Trying fallback URLs from config...");
            if let Ok(stream) = self.connect_multiple(state.urls.clone(), creds.clone()).await {
                return Ok(stream);
            }
        }

        let urls = creds
            .servers()
            .await
            .map_err(|e| ConnectorError::Core(e.to_string()))?;
        self.connect_multiple(urls, creds).await
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        // Implement disconnect logic if needed
        warn!(target: "PocketConnect", "Disconnect method is not implemented yet and shouldn't be called.");
        Ok(())
    }
}

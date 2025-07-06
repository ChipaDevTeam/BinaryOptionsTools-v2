use std::sync::Arc;

use binary_options_tools_core_pre::{connector::{Connector, ConnectorError, ConnectorResult}, reimports::{MaybeTlsStream, WebSocketStream}};
use futures_util::stream::FuturesUnordered;
use tokio::net::TcpStream;
use tracing::{info, warn};

use futures_util::StreamExt;
use crate::{pocketoption::utils::connect::try_connect2, pocketoption_pre::{ssid::Ssid, state::State}};

#[derive(Clone)]
pub struct PocketConnect;

impl PocketConnect {
    async fn connect_multiple(&self, url: Vec<String>, ssid: Ssid) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let mut futures = FuturesUnordered::new();
        for u in url {
            futures.push(
                async {
                    info!(target: "PocketConnectThread", "Connecting to PocketOption at {}", u);
                    try_connect2(ssid.clone(), u.clone()).await.map_err(|e | (e, u))
                }
            );
        }
        while let Some(result) = futures.next().await {
            match result {
                Ok(stream) => {
                    info!(target: "PocketConnect", "Successfully connected to PocketOption");
                    return Ok(stream);
                },
                Err((e, u)) => warn!(target: "PocketConnect", "Failed to connect to {}: {}", u, e),
            }
        }
        Err(ConnectorError::Custom("Failed to connect to any of the provided URLs".to_string()))
    }
}


#[async_trait::async_trait]
impl Connector<State> for PocketConnect {
    async fn connect(&self, state: Arc<State>) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let creds = state.ssid.clone();
        let url = state.default_connection_url.clone();
        if let Some(url) = url {
            info!(target: "PocketConnect", "Connecting to PocketOption at {}", url);
            return try_connect2(creds, url).await.map_err(|e| ConnectorError::Custom(e.to_string()));
        }
        let urls = creds.servers().await.map_err(|e| ConnectorError::Core(e.to_string()))?;
        self.connect_multiple(urls, creds).await
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        // Implement disconnect logic if needed
        warn!(target: "PocketConnect", "Disconnect method is not implemented yet and shouldn't be called.");
        Ok(())
    }
}

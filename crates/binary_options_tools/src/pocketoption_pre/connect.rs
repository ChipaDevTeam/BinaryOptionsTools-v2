use std::sync::Arc;

use binary_options_tools_core_pre::{connector::{Connector, ConnectorError, ConnectorResult}, reimports::{MaybeTlsStream, WebSocketStream}};
use tokio::net::TcpStream;
use tracing::{info, warn};

use crate::{pocketoption::utils::connect::try_connect2, pocketoption_pre::state::State};

#[derive(Clone)]
pub struct PocketConnect;

#[async_trait::async_trait]
impl Connector<State> for PocketConnect {
    async fn connect(&self, state: Arc<State>) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let creds = state.ssid.clone();
        let url = state.default_connection_url.clone();
        if let Some(url) = url {
            info!(target: "PocketConnect", "Connecting to PocketOption at {}", url);
            return try_connect2(creds, url).await.map_err(|e| ConnectorError::Custom(e.to_string()));
        }
        let url = creds.server().await.map_err(|e| ConnectorError::Core(e.to_string()))?;
        info!(target: "PocketConnect", "Connecting to PocketOption at {}", url);
        try_connect2(creds, url).await.map_err(|e| ConnectorError::Custom(e.to_string()))
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        // Implement disconnect logic if needed
        warn!(target: "PocketConnect", "Disconnect method is not implemented yet and shouldn't be called.");
        Ok(())
    }
}

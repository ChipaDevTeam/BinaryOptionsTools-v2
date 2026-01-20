use std::sync::Arc;

use binary_options_tools_core_pre::{
    connector::{Connector as ConnectorTrait, ConnectorError, ConnectorResult},
    reimports::{
        Connector, MaybeTlsStream, Request, WebSocketStream, connect_async_tls_with_config,
        generate_key,
    },
};
use futures_util::{StreamExt, stream::FuturesUnordered};
use tokio::net::TcpStream;
use tracing::{info, warn};
use url::Url;

use crate::expertoptions::{regions::Regions, state::State};

#[derive(Clone)]
pub struct ExpertConnect;

#[async_trait::async_trait]
impl ConnectorTrait<State> for ExpertConnect {
    async fn connect(
        &self,
        state: Arc<State>,
    ) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        // Implement connection logic here
        let mut futures = FuturesUnordered::new();
        let url = Regions::regions_str().into_iter().map(String::from); // No demo region for ExpertOptions
        for u in url {
            futures.push(async {
                info!(target: "ExpertConnectThread", "Connecting to ExpertOptions at {u}");
                try_connect(state.user_agent().await, u.clone())
                    .await
                    .map_err(|e| (e, u))
            });
        }
        while let Some(result) = futures.next().await {
            match result {
                Ok(stream) => {
                    info!(target: "PocketConnect", "Successfully connected to ExpertOptions");
                    return Ok(stream);
                }
                Err((e, u)) => warn!(target: "PocketConnect", "Failed to connect to {}: {}", u, e),
            }
        }
        Err(ConnectorError::Custom(
            "Failed to connect to any of the provided URLs".to_string(),
        ))
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        // Implement disconnect logic if needed
        warn!(target: "ExpertConnect", "Disconnect method is not implemented yet and shouldn't be called.");
        Ok(())
    }
}

pub async fn try_connect(
    agent: String,
    url: String,
) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let mut root_store = rustls::RootCertStore::empty();
    let certs_result = rustls_native_certs::load_native_certs();
    for cert in certs_result.certs {
        root_store.add(cert).ok();
    }
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = Connector::Rustls(std::sync::Arc::new(tls_config));

    let t_url = Url::parse(&url).map_err(|e| ConnectorError::UrlParsing(e.to_string()))?;
    let host = t_url
        .host_str()
        .ok_or(ConnectorError::UrlParsing("Host not found".into()))?;
    let request = Request::builder()
        .uri(t_url.to_string())
        .header("Origin", "https://app.expertoption.com")
        .header("Cache-Control", "no-cache")
        .header("User-Agent", agent)
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13")
        .header("Host", host)
        .body(())
        .map_err(|e| ConnectorError::HttpRequestBuild(e.to_string()))?;

    let (ws, _) = connect_async_tls_with_config(request, None, false, Some(connector))
        .await
        .map_err(|e| ConnectorError::Custom(e.to_string()))?;
    Ok(ws)
}

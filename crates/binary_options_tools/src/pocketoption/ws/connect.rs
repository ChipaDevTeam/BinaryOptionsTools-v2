use std::time::Duration;

use async_channel::{Sender, bounded};
use async_trait::async_trait;
use futures_util::future::join_all;
use tokio::net::TcpStream;
use tracing::info;
use url::Url;

use crate::pocketoption::{error::PocketOptionError, utils::connect::try_connect};
use binary_options_tools_core::{
    error::{BinaryOptionsResult, BinaryOptionsToolsError},
    general::{
        config::Config,
        traits::{Connect, DataHandler, InnerConfig, MessageTransfer},
    },
    reimports::{MaybeTlsStream, WebSocketStream},
};

use super::ssid::Ssid;

#[derive(Clone)]
pub struct PocketConnect;

#[async_trait]
impl Connect for PocketConnect {
    type Creds = Ssid;

    async fn connect<T: DataHandler, Transfer: MessageTransfer, U: InnerConfig>(
        &self,
        creds: Self::Creds,
        config: &Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        async fn send_ws(
            creds: Ssid,
            url: String,
            sender: Sender<(WebSocketStream<MaybeTlsStream<TcpStream>>, String)>,
        ) -> BinaryOptionsResult<()> {
            info!(target: "TryConnect", "Trying to connecto to {}", url);
            if let Ok(connect) = try_connect(creds, url.clone()).await {
                info!(target: "SuccessConnect", "Succesfully connected to {}", url);
                sender.send((connect, url.clone())).await.map_err(|e| {
                    BinaryOptionsToolsError::GeneralMessageSendingError(e.to_string())
                })?;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            Err(BinaryOptionsToolsError::WebsocketRecievingConnectionError(
                url,
            ))
        }
        let (sender, reciever) = bounded(1); // It should stop after recieving only one message
        let default_urls = config.get_default_connection_url()?;
        let default_connections = default_urls
            .iter()
            .map(|url| tokio::spawn(send_ws(creds.clone(), url.to_string(), sender.clone())));
        tokio::select! {
            res = reciever.recv() => return Ok(res.map(|(r, _)| r)?),
            _ = join_all(default_connections) => {}
        }
        let urls = creds
            .servers()
            .await
            .map_err(|e| PocketOptionError::SsidParsingError(e.to_string()))?;
        let connections = urls
            .iter()
            .map(|url| tokio::spawn(send_ws(creds.clone(), url.to_owned(), sender.clone())));
        tokio::select! {
            res = reciever.recv() => match res {
                Ok((res, url)) => {
                    config.add_default_connection_url(Url::parse(&url)?)?;
                    Ok(res)
                },
                Err(e) => Err(e.into())
            },
            _ = join_all(connections) => Err(
                        PocketOptionError::WebsocketMultipleAttemptsConnectionError(format!(
                            "Couldn't connect to server after {} attempts.",
                            urls.len()
                        ))
                        .into(),
                    )
        }
    }
}

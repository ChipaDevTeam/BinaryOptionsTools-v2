use std::time::Duration;

use async_trait::async_trait;
use futures_util::future::try_join;
use tokio::time::sleep;
use tracing::{debug, info, instrument};

use crate::pocketoption::{parser::message::WebSocketMessage, types::info::MessageInfo};
use binary_options_tools_core::{
    error::{BinaryOptionsResult, BinaryOptionsToolsError},
    general::{config::Config, send::SenderMessage, traits::WCallback, types::Data},
};

use super::{base::ChangeSymbol, data::PocketData, order::SuccessCloseOrder};

#[derive(Clone)]
pub struct PocketCallback;

impl PocketCallback {
    async fn update_assets(
        data: &Data<PocketData, WebSocketMessage>,
        sender: &SenderMessage,
    ) -> BinaryOptionsResult<()> {
        for asset in data.stream_assets().await {
            // Send 3 messages, 1: change symbol, 2: unsubscribe symbol, 3: subscribe symbol
            debug!("Updating asset: {asset}");
            sender
                .send(WebSocketMessage::ChangeSymbol(ChangeSymbol::new(
                    asset.to_string(),
                    1,
                )))
                .await?;
            sender
                .send(WebSocketMessage::Unsubfor(asset.to_string()))
                .await?;
            sender
                .send(WebSocketMessage::Subfor(asset.to_string()))
                .await?;
            sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    async fn update_check_results(
        data: &Data<PocketData, WebSocketMessage>,
    ) -> BinaryOptionsResult<()> {
        if let Some(sender) = data.sender(MessageInfo::SuccesscloseOrder).await {
            let deals = data.get_closed_deals().await;
            if !deals.is_empty() {
                info!(target: "CheckResultCallback", "Sending closed orders data after disconnection");
                let close_order = SuccessCloseOrder { profit: 0.0, deals };
                sender
                    .send(WebSocketMessage::SuccesscloseOrder(close_order))
                    .await
                    .map_err(|e| {
                        BinaryOptionsToolsError::GeneralMessageSendingError(e.to_string())
                    })?;
            }
        }
        Ok(())
    }
}
#[async_trait]
impl WCallback for PocketCallback {
    type T = PocketData;
    type Transfer = WebSocketMessage;
    type U = ();

    #[instrument(skip(self, data, sender, _config))]
    async fn call(
        &self,
        data: Data<Self::T, Self::Transfer>,
        sender: &SenderMessage,
        _config: &Config<Self::T, Self::Transfer, Self::U>,
    ) -> BinaryOptionsResult<()> {
        // let sender = sender.clone();
        let update_assets_future = Self::update_assets(&data, sender);
        let update_check_results_future = Self::update_check_results(&data);
        try_join(update_assets_future, update_check_results_future).await?;
        Ok(())
    }
}

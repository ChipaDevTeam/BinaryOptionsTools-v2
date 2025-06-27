use std::time::Duration;

use async_trait::async_trait;
use futures_util::future::try_join;
use tokio::time::sleep;
use tracing::{debug, info, instrument};

use crate::pocketoption::{
    error::PocketOptionError, parser::message::WebSocketMessage, types::info::MessageInfo,
    validators::history_validator,
};
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
        config: &Config<PocketData, WebSocketMessage, ()>,
    ) -> BinaryOptionsResult<()> {
        for asset in data.stream_assets().await {
            sleep(Duration::from_secs(1)).await;
            let history = ChangeSymbol::new(asset.to_string(), 3600);
            let res = sender
                .send_message_with_timout(
                    config.get_timeout()?,
                    "SubscribeSymbolCallback",
                    data,
                    WebSocketMessage::ChangeSymbol(history),
                    MessageInfo::UpdateHistoryNew,
                    &history_validator(asset.to_string(), 3600),
                )
                .await?;
            if let WebSocketMessage::UpdateHistoryNew(_) = res {
                debug!("Sent 'ChangeSymbol' for asset: {asset}");
            } else {
                return Err(PocketOptionError::UnexpectedIncorrectWebSocketMessage(
                    res.information(),
                )
                .into());
            }
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

    #[instrument(skip(self, data, sender, config))]
    async fn call(
        &self,
        data: Data<Self::T, Self::Transfer>,
        sender: &SenderMessage,
        config: &Config<Self::T, Self::Transfer, Self::U>,
    ) -> BinaryOptionsResult<()> {
        // let sender = sender.clone();
        let update_assets_future = Self::update_assets(&data, sender, config);
        let update_check_results_future = Self::update_check_results(&data);
        try_join(update_assets_future, update_check_results_future).await?;
        Ok(())
    }
}

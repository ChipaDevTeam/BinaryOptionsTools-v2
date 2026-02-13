use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};

use crate::pocketoption::{state::State, types::TwoStepRule};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceMessage {
    balance: Decimal,
    #[serde(flatten)]
    _extra: HashMap<String, Value>,
}

pub struct BalanceModule {
    state: Arc<State>,
    receiver: AsyncReceiver<Arc<Message>>,
}

#[async_trait]
impl LightweightModule<State> for BalanceModule {
    fn new(
        state: Arc<State>,
        _: AsyncSender<Message>,
        receiver: AsyncReceiver<Arc<Message>>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            match &*msg {
                Message::Binary(data) => {
                    if let Ok(balance_msg) = serde_json::from_slice::<BalanceMessage>(data) {
                        debug!("Received balance message (binary): {:?}", balance_msg);
                        self.state.set_balance(balance_msg.balance).await;
                    } else {
                        warn!("Failed to parse balance message (binary): {:?}", data);
                    }
                }
                Message::Text(text) => {
                    if let Ok(balance_msg) = serde_json::from_str::<BalanceMessage>(text) {
                        debug!("Received balance message (text): {:?}", balance_msg);
                        self.state.set_balance(balance_msg.balance).await;
                    } else if let Some(start) = text.find('[') {
                        // Try to parse as a 1-step Socket.IO message: 42["successupdateBalance", {...}]
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text[start..])
                        {
                            if let Some(arr) = value.as_array() {
                                if arr.len() >= 2 && arr[0] == "successupdateBalance" {
                                    if let Ok(balance_msg) =
                                        serde_json::from_value::<BalanceMessage>(arr[1].clone())
                                    {
                                        debug!(
                                            "Received balance message (text 1-step): {:?}",
                                            balance_msg
                                        );
                                        self.state.set_balance(balance_msg.balance).await;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Err(CoreError::LightweightModuleLoop("BalanceModule".into()))
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(TwoStepRule::new(r#"451-["successupdateBalance","#))
    }
}

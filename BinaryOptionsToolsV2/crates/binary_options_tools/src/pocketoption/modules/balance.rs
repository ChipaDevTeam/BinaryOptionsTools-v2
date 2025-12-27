use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule},
};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};

use crate::pocketoption::{state::State, types::TwoStepRule};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceMessage {
    balance: f64,
    #[serde(flatten)]
    _extra: HashMap<String, Value>,
}

// #[derive(Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct DemoBalance {
//     is_demo: u8,
//     balance: f64,
// }

// #[derive(Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct LiveBalance {
//     uid: u64,
//     login: u64,
//     is_demo: u8,
//     balance: f64,
// }

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
    ) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            if let Message::Binary(text) = &*msg {
                if let Ok(balance_msg) = serde_json::from_slice::<BalanceMessage>(text) {
                    debug!("Received balance message: {:?}", balance_msg);
                    self.state.set_balance(balance_msg.balance).await;
                    // If you want to handle demo/live balance differently, you can add logic here
                    // For example, if you had a field to distinguish between demo and live:
                    // if balance_msg.is_demo == 1 {
                    //     self.state.set_demo_balance(balance_msg.balance);
                    // } else {
                    //     self.state.set_live_balance(balance_msg.balance);
                    // }
                } else {
                    warn!("Failed to parse balance message: {:?}", text);
                }
            }
        }
        Err(CoreError::LightweightModuleLoop("BalanceModule".into()))
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(TwoStepRule::new(r#"451-["successupdateBalance","#))
    }
}

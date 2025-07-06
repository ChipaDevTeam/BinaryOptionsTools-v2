use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

use async_trait::async_trait;
use binary_options_tools_core::reimports::Message;
use binary_options_tools_core_pre::{error::{CoreError, CoreResult}, reimports::{AsyncReceiver, AsyncSender}, traits::{ApiModule, LightweightModule, Rule}};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::pocketoption_pre::state::State;

#[derive(Debug, Deserialize)]
#[serde(untagged)] // Allows matching against different struct patterns
enum BalanceMessage {
    Demo(DemoBalance),
    Live(LiveBalance),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DemoBalance {
    is_demo: u8,
    balance: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveBalance {
    uid: u64,
    login: u64,
    is_demo: u8,
    balance: f64,
}


struct BalanceRule {
    is_balance: AtomicBool
}

pub struct BalanceModule {
    state: Arc<State>,
    receiver: AsyncReceiver<Arc<Message>>,
}

#[async_trait]
impl LightweightModule<State> for BalanceModule {
    fn new(state: Arc<State>, _: AsyncSender<Message>, receiver: AsyncReceiver<Arc<Message>>) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            if let Message::Binary(text) = &*msg {
                if let Ok(balance_msg) = serde_json::from_slice::<BalanceMessage>(text) {
                    match balance_msg {
                        BalanceMessage::Demo(DemoBalance {balance, .. }) | BalanceMessage::Live(LiveBalance { balance, ..})=> {
                            debug!("Received balance update: {}", balance);
                            let mut state = self.state.balance.write().map_err(|e| CoreError::Poison(e.to_string()))?;
                            *state = Some(balance);
                        }
                    }
                } else {
                    warn!("Failed to parse balance message: {:?}", text);
                }
            }
        }
        Ok(())
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(BalanceRule {
            is_balance: AtomicBool::new(false),
        })
    }
} 

impl Rule for BalanceRule {
    fn call(&self, msg: &Message) -> bool {
        match msg {
            Message::Text(text) => {
                if text.starts_with(r#"451-["successupdateBalance","#) {
                    self.is_balance.store(true, Ordering::SeqCst);
                }
                false
            },
            Message::Binary(_) if self.is_balance.load(Ordering::SeqCst) => {
                self.is_balance.store(false, Ordering::SeqCst);
                true
            },
            _ => false
        }
    }

    fn reset(&self) {
        self.is_balance.store(false, Ordering::SeqCst);
    }
}

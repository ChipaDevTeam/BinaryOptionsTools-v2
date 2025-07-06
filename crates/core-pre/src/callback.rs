use std::sync::Arc;

use async_trait::async_trait;
use kanal::AsyncSender;
use tokio_tungstenite::tungstenite::Message;

use crate::{error::CoreResult, traits::{AppState, ReconnectCallback}};

pub struct ConnectionCallback<S: AppState> {
    pub on_connect: OnConnectCallback<S>,
    pub on_reconnect: ReconnectCallbackStack<S>,
}

// --- Callbacks and Lightweight Handlers ---
pub type OnConnectCallback<S> = Box<
    dyn Fn(
            Arc<S>,
            &AsyncSender<Message>,
        ) -> futures_util::future::BoxFuture<'static, CoreResult<()>>
        + Send
        + Sync,
>;


pub struct ReconnectCallbackStack<S: AppState> {
    pub layers: Vec<Box<dyn ReconnectCallback<S>>>,
}

impl<S: AppState> Default for ReconnectCallbackStack<S> {
    fn default() -> Self {
        Self { layers: Vec::new() }
    }
}

impl<S: AppState> ReconnectCallbackStack<S> {
    pub fn add_layer(&mut self, layer: Box<dyn ReconnectCallback<S>>) {
        self.layers.push(layer);
    }
}

#[async_trait]
impl<S: AppState> ReconnectCallback<S> for ReconnectCallbackStack<S> {
    async fn call(
        &self,
        state: Arc<S>,
        sender: &AsyncSender<Message>,
    ) -> CoreResult<()> {
        for layer in &self.layers {
            layer.call(state.clone(), sender).await?;
        }
        Ok(())
    }
}
use std::sync::Arc;

use binary_options_tools_core_pre::{builder::ClientBuilder, client::Client};
use tokio::task::JoinHandle;

use crate::expertoptions::{connect::ExpertConnect, error::ExpertOptionsResult, modules::keep_alive::PongModule, state::State};

#[derive(Clone)]

pub struct ExpertOptions {
    client: Client<State>,
    _runner: Arc<JoinHandle<()>>,
}

impl ExpertOptions {
    fn builder(token: impl ToString, demo: bool) -> ExpertOptionsResult<ClientBuilder<State>> {
        let state = State::new(token.to_string(), demo);

        Ok(ClientBuilder::new(ExpertConnect, state)
            .with_lightweight_module::<PongModule>()
        )
    }

    pub async fn new(token: impl ToString, demo: bool) -> ExpertOptionsResult<Self> {
        let builder = Self::builder(token, demo)?;
        let (client, mut runner) = builder.build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });
        client.wait_connected().await;

        Ok(Self {
            client,
            _runner: Arc::new(_runner),
        })
    }

}
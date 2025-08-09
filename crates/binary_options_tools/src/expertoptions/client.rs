use std::sync::Arc;

use binary_options_tools_core_pre::{builder::ClientBuilder, client::Client};
use tokio::task::JoinHandle;

use crate::{
    expertoptions::{
        connect::ExpertConnect,
        error::ExpertOptionsResult,
        modules::{keep_alive::PongModule, profile::ProfileModule},
        state::State,
    },
    utils::{PrintMiddleware, print_handler},
};

#[derive(Clone)]

pub struct ExpertOptions {
    client: Client<State>,
    _runner: Arc<JoinHandle<()>>,
}

impl ExpertOptions {
    fn builder(token: impl ToString, demo: bool) -> ExpertOptionsResult<ClientBuilder<State>> {
        let state = State::new(token.to_string(), demo);

        Ok(ClientBuilder::new(ExpertConnect, state)
            .with_middleware(Box::new(PrintMiddleware))
            // .with_lightweight_handler(|msg, _, _| Box::pin(print_handler(msg)))
            .with_lightweight_module::<PongModule>()
            .with_module::<ProfileModule>())
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_expert_options_connection() {
        tracing_subscriber::fmt::init();

        let token = "759c67788715ca4e2e64c9ebb39e1c65";
        let demo = true;

        let expert_options = ExpertOptions::new(token, demo).await;

        assert!(expert_options.is_ok());

        tokio::time::sleep(Duration::from_secs(30)).await;
        println!("Test completed, connection should be stable.");
    }
}

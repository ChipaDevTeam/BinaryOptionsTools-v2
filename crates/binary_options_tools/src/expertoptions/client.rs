use std::{sync::Arc, time::Duration};

use binary_options_tools_core_pre::{
    builder::ClientBuilder,
    client::Client,
    error::CoreError,
    testing::{TestingWrapper, TestingWrapperBuilder},
};
use tokio::task::JoinHandle;

use crate::{
    expertoptions::{
        connect::ExpertConnect,
        error::{ExpertOptionsError, ExpertOptionsResult},
        modules::{keep_alive::PongModule, profile::ProfileModule},
        state::State,
    },
    utils::PrintMiddleware,
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

    /// Switches the client to a different account type (demo or real).
    /// if demo is true then changes to demo, otherwhise to real account
    pub async fn set_context(&self, demo: bool) -> ExpertOptionsResult<()> {
        if let Some(handle) = self.client.get_handle::<ProfileModule>().await {
            Ok(handle.set_context(demo).await?)
        } else {
            Err(CoreError::ModuleNotFound("ProfileModule".into()).into())
        }
    }

    /// Checks if the current account is a demo account.
    pub async fn is_demo(&self) -> bool {
        self.client.state.is_demo().await
    }

    /// Disconnects and reconnects the client.
    pub async fn reconnect(&self) -> ExpertOptionsResult<()> {
        self.client
            .reconnect()
            .await
            .map_err(ExpertOptionsError::from)
    }

    /// Shuts down the client and stops the runner.
    pub async fn shutdown(self) -> ExpertOptionsResult<()> {
        self.client
            .shutdown()
            .await
            .map_err(ExpertOptionsError::from)
    }

    pub async fn new_testing_wrapper(
        token: impl ToString,
        demo: bool,
    ) -> ExpertOptionsResult<TestingWrapper<State>> {
        let pocket_builder = Self::builder(token, demo)?;
        let builder = TestingWrapperBuilder::new()
            .with_stats_interval(Duration::from_secs(10))
            .with_log_stats(true)
            .with_track_events(true)
            .with_max_reconnect_attempts(Some(3))
            .with_reconnect_delay(Duration::from_secs(5))
            .with_connection_timeout(Duration::from_secs(30))
            .with_auto_reconnect(true)
            .build_with_middleware(pocket_builder)
            .await?;

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_expert_options_connection() {
        let _ = tracing_subscriber::fmt::try_init();

        let token = "759c67788715ca4e2e64c9ebb39e1c65";
        let demo = true;

        let expert_options = ExpertOptions::new(token, demo).await;

        assert!(expert_options.is_ok());

        tokio::time::sleep(Duration::from_secs(30)).await;
        println!("Test completed, connection should be stable.");
    }

    #[tokio::test]
    async fn test_expert_options_change_account_type() {
        let token = "759c67788715ca4e2e64c9ebb39e1c65";
        let demo = true;

        let expert_options = ExpertOptions::new(token, demo).await;

        assert!(expert_options.is_ok());

        let expert_options = expert_options.unwrap();
        dbg!("ExpertOptions created successfully");
        tokio::time::sleep(Duration::from_secs(5)).await;
        // Change to real account
        expert_options.set_context(false).await.unwrap();
        assert!(!expert_options.is_demo().await);
        dbg!("Changed to real account successfully");
        tokio::time::sleep(Duration::from_secs(5)).await;
        // Change back to demo account
        expert_options.set_context(true).await.unwrap();
        assert!(expert_options.is_demo().await);
    }
}

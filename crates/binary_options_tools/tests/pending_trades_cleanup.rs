use binary_options_tools::pocketoption::modules::pending_trades::{
    Command, CommandResponse, PendingTradesApiModule,
};
use binary_options_tools::pocketoption::ssid::Ssid;
use binary_options_tools::pocketoption::state::StateBuilder;
use binary_options_tools_core::reimports::bounded_async;
use binary_options_tools_core::traits::ApiModule;
use kanal::unbounded_async;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_pending_trades_cleanup_on_stop() {
    let result = timeout(Duration::from_secs(5), async {
        let ssid_json = r#"{"session":"mock_session_id","isDemo":1,"uid":12345,"platform":2}"#;
        let ssid = Ssid::parse(ssid_json).expect("Failed to parse mock SSID");
        let state = Arc::new(StateBuilder::default().ssid(ssid).build().unwrap());

        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (cmd_resp_tx, cmd_resp_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);
        let (ws_sender_tx, _ws_sender_rx) = unbounded_async();
        let (runner_tx, _runner_rx) = bounded_async(10);

        let mut module = PendingTradesApiModule::new(
            state.clone(),
            cmd_rx,
            cmd_resp_tx,
            ws_rx,
            ws_sender_tx,
            runner_tx,
        );

        // Spawn the module
        let module_handle = tokio::spawn(async move { module.run().await });

        // Send cancel command directly (bypass handle to avoid call_lock hang)
        let _ = cmd_tx
            .send(Command::CancelPendingOrder {
                ticket: "test_ticket".to_string(),
                req_id: uuid::Uuid::new_v4(),
            })
            .await;

        // Give it a moment to register the waiter
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Stop the module by dropping the command sender
        drop(cmd_tx);
        drop(ws_tx);

        // The module should finish and we should receive Shutdown
        let response = timeout(Duration::from_secs(5), cmd_resp_rx.recv())
            .await
            .expect("Timed out waiting for shutdown")
            .expect("Channel closed unexpectedly");

        match response {
            CommandResponse::Shutdown { .. } => {}
            other => panic!("Expected Shutdown response, got {:?}", other),
        }

        module_handle.await.unwrap().unwrap();
    })
    .await;

    assert!(result.is_ok(), "Test timed out");
}

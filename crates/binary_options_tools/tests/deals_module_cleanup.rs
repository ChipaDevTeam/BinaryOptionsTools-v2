use binary_options_tools::pocketoption::error::PocketError;
use binary_options_tools::pocketoption::modules::deals::DealsApiModule;
use binary_options_tools::pocketoption::ssid::Ssid;
use binary_options_tools::pocketoption::state::StateBuilder;
use binary_options_tools::pocketoption::types::Deal;
use binary_options_tools_core::reimports::bounded_async;
use binary_options_tools_core::traits::ApiModule;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_deals_module_cleanup_on_stop() {
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let ssid_json = r#"{"session":"mock_session_id","isDemo":1,"uid":12345,"platform":2}"#;
        let ssid = Ssid::parse(ssid_json).expect("Failed to parse mock SSID");
        let state = Arc::new(StateBuilder::default().ssid(ssid).build().unwrap());

        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (cmd_resp_tx, cmd_resp_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);
        let (ws_sender_tx, _ws_sender_rx) = bounded_async(10);
        let (runner_tx, _runner_rx) = bounded_async(10);

        let mut module = DealsApiModule::new(
            state.clone(),
            cmd_rx,
            cmd_resp_tx,
            ws_rx,
            ws_sender_tx,
            runner_tx,
        );

        let handle = DealsApiModule::create_handle(cmd_tx.clone(), cmd_resp_rx);

        let trade_id = Uuid::new_v4();

        // Create a mock deal and add it to opened_deals
        let deal_json = format!(
            r#"{{
            "id": "{}",
            "openTime": "2023-01-01 00:00:00",
            "closeTime": "2023-01-01 00:01:00",
            "openTimestamp": 1672531200,
            "closeTimestamp": 1672531260,
            "uid": 12345,
            "amount": "100.0",
            "profit": "80.0",
            "percentProfit": 80,
            "percentLoss": 0,
            "openPrice": "1.0850",
            "closePrice": "1.0860",
            "command": 1,
            "asset": "EURUSD_otc",
            "isDemo": 1,
            "copyTicket": "",
            "openMs": 123,
            "optionType": 1,
            "currency": "USD"
        }}"#,
            trade_id
        );
        let deal: Deal = serde_json::from_str(&deal_json).unwrap();
        state.trade_state.add_opened_deal(deal).await;

        // Spawn the module
        let module_handle = tokio::spawn(async move { module.run().await });

        // Request check_result which should wait
        let wait_handle = tokio::spawn(async move { handle.check_result(trade_id).await });

        // Give it a moment to register the waiter
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Stop the module by dropping the websocket sender (more reliable in this test)
        drop(ws_tx);
        drop(cmd_tx);

        // The module should finish and the waiter should receive an error
        let result = wait_handle.await.unwrap();

        match result {
            Err(PocketError::ModuleStopped { module_name, .. }) => {
                assert_eq!(module_name, "DealsApiModule");
            }
            other => panic!("Expected ModuleStopped error, got {:?}", other),
        }

        module_handle.await.unwrap().unwrap();
    })
    .await;

    assert!(result.is_ok(), "Test timed out after 5 seconds");
}

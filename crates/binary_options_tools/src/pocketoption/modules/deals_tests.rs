#[cfg(test)]
mod tests {
    use crate::pocketoption::{
        modules::deals::{Command, CommandResponse, DealsApiModule, DealsHandle},
        state::TradeState,
        types::Deal,
    };
    use binary_options_tools_core::{
        reimports::{AsyncReceiver, AsyncSender, Message},
        traits::{ApiModule, RunnerCommand},
    };
    use kanal::bounded_async;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::oneshot;
    use uuid::Uuid;

    // Helper to create a mock deal
    fn create_mock_deal(id: Uuid) -> Deal {
        let json = json!({
            "id": id,
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
        });
        serde_json::from_value(json).unwrap()
    }

    #[tokio::test]
    async fn test_check_result_already_closed() {
        // Setup state with a closed deal
        let trade_state = Arc::new(TradeState::default());
        let deal_id = Uuid::new_v4();
        let deal = create_mock_deal(deal_id);
        trade_state.update_closed_deals(vec![deal.clone()]).await;

        let state = Arc::new(
            crate::pocketoption::state::StateBuilder::default()
                .ssid(
                    crate::pocketoption::ssid::Ssid::parse(
                        "{\"session\":\"test\",\"isDemo\":1,\"uid\":123,\"platform\":2}",
                    )
                    .unwrap(),
                )
                .build_with_trade_state(trade_state)
                .unwrap(),
        );

        let (ws_tx, ws_rx) = bounded_async::<Arc<Message>>(1);
        let (cmd_tx, cmd_rx) = bounded_async::<Command>(1);
        let (res_tx, res_rx) = bounded_async::<CommandResponse>(1);
        let (ws_sender_tx, _ws_sender_rx) = bounded_async::<Message>(1);
        let (runner_tx, _runner_rx) = bounded_async::<RunnerCommand>(1);

        let mut module = DealsApiModule::new(state, cmd_rx, res_tx, ws_rx, ws_sender_tx, runner_tx);

        // Simulate CheckResult command
        let (tx, rx) = oneshot::channel();
        cmd_tx
            .send(Command::CheckResult(deal_id, tx))
            .await
            .unwrap();

        // Run module for a bit
        let _ = tokio::time::timeout(tokio::time::Duration::from_millis(10), module.run()).await;

        let result = rx.await.unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, deal_id);
    }

    #[tokio::test]
    async fn test_check_result_waits_for_close() {
        let trade_state = Arc::new(TradeState::default());
        let deal_id = Uuid::new_v4();
        let deal = create_mock_deal(deal_id);

        // Put in opened_deals first
        trade_state.add_opened_deal(deal.clone()).await;

        let state = Arc::new(
            crate::pocketoption::state::StateBuilder::default()
                .ssid(
                    crate::pocketoption::ssid::Ssid::parse(
                        "{\"session\":\"test\",\"isDemo\":1,\"uid\":123,\"platform\":2}",
                    )
                    .unwrap(),
                )
                .build_with_trade_state(trade_state)
                .unwrap(),
        );

        let (ws_tx, ws_rx) = bounded_async::<Arc<Message>>(10);
        let (cmd_tx, cmd_rx) = bounded_async::<Command>(10);
        let (res_tx, res_rx) = bounded_async::<CommandResponse>(10);
        let (ws_sender_tx, _ws_sender_rx) = bounded_async::<Message>(1);
        let (runner_tx, _runner_rx) = bounded_async::<RunnerCommand>(1);

        let mut module = DealsApiModule::new(state, cmd_rx, res_tx, ws_rx, ws_sender_tx, runner_tx);

        // Start CheckResult
        let (tx, rx) = oneshot::channel();
        cmd_tx
            .send(Command::CheckResult(deal_id, tx))
            .await
            .unwrap();

        // Spawn module run
        let module_handle = tokio::spawn(async move { module.run().await });

        // Small delay to ensure command is processed
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Simulate WebSocket event "updateClosedDeals"
        let event = json!(["updateClosedDeals", [deal.clone()]]);
        let msg = format!("42{}", serde_json::to_string(&event).unwrap());
        ws_tx
            .send(Arc::new(Message::Text(msg.into())))
            .await
            .unwrap();

        // Verify result
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, deal_id);

        module_handle.abort();
    }
}

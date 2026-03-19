#[cfg(test)]
mod tests {
    use crate::pocketoption::modules::balance::BalanceModule;
    use crate::pocketoption::modules::deals::DealsApiModule;
    use crate::pocketoption::ssid::Ssid;
    use crate::pocketoption::state::StateBuilder;
    use binary_options_tools_core::reimports::{bounded_async, Message};
    use binary_options_tools_core::traits::{ApiModule, LightweightModule};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_balance_module_resilient_parsing() {
        let dummy_ssid = r#"42["auth",{"session":"dummy","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid).unwrap();
        let state = Arc::new(StateBuilder::default().ssid(ssid).build().unwrap());

        let (_ws_tx, _ws_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (runner_tx, _runner_rx) = bounded_async(1);

        let mut module = BalanceModule::new(state.clone(), _ws_tx, msg_rx, runner_tx);
        let rule = BalanceModule::rule();

        // 1. Test 451- prefix (legacy/binary)
        let msg_451 = Message::text(r#"451-["successupdateBalance",{"balance":123.45}]"#);
        assert!(rule.call(&msg_451), "Rule should match 451- prefix");

        // 2. Test 42 prefix (standard)
        let msg_42 = Message::text(r#"42["successupdateBalance",{"balance":678.90}]"#);
        assert!(rule.call(&msg_42), "Rule should match 42 prefix");

        // Run the module in background
        tokio::spawn(async move {
            let _ = module.run().await;
        });

        // Send 451 message
        msg_tx.send(Arc::new(msg_451)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(*state.balance.read().await, Some(dec!(123.45)));

        // Send 42 message
        msg_tx.send(Arc::new(msg_42)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(*state.balance.read().await, Some(dec!(678.90)));
    }

    #[tokio::test]
    async fn test_deals_module_resilient_parsing() {
        let dummy_ssid = r#"42["auth",{"session":"dummy","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid).unwrap();
        let state = Arc::new(StateBuilder::default().ssid(ssid).build().unwrap());

        let (_cmd_tx, _cmd_rx) = bounded_async(10);
        let (resp_tx, _resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, _ws_rx) = bounded_async(10);
        let (runner_tx, _runner_rx) = bounded_async(1);

        let mut module =
            DealsApiModule::new(state.clone(), _cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);
        let rule = DealsApiModule::rule(state.clone());

        // Test patterns
        let msg_451 = Message::text(r#"451-["updateOpenedDeals",[]]"#);
        assert!(
            rule.call(&msg_451),
            "Rule should match 451- prefix for deals"
        );

        let msg_42 = Message::text(r#"42["updateOpenedDeals",[]]"#);
        assert!(rule.call(&msg_42), "Rule should match 42 prefix for deals");

        // Run module
        tokio::spawn(async move {
            let _ = module.run().await;
        });

        // Verify that 42["updateOpenedDeals", [...]] is correctly parsed
        // We'll send a real deal in 42 format
        let deal_json = r#"{
            "id": "2f561661-334c-4de3-920f-f095c7b1193f",
            "openTime": "2024-12-05 00:52:26",
            "closeTime": "2024-12-05 01:22:26",
            "openTimestamp": 1733359946,
            "closeTimestamp": 1733361746,
            "uid": 87742848,
            "amount": 1,
            "profit": 0.87,
            "percentProfit": 87,
            "percentLoss": 100,
            "openPrice": 37.81371,
            "closePrice": 0,
            "command": 1,
            "asset": "EURTRY_otc",
            "isDemo": 1,
            "copyTicket": "",
            "openMs": 61,
            "optionType": 100,
            "isRollover": false,
            "isCopySignal": false,
            "currency": "USD",
            "amountUSD": 1
        }"#;
        let msg_deal_42 = Message::text(format!(r#"42["updateOpenedDeals",[{}]]"#, deal_json));

        msg_tx.send(Arc::new(msg_deal_42)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let opened = state.trade_state.opened_deals.read().await;
        let deal_id = uuid::Uuid::parse_str("2f561661-334c-4de3-920f-f095c7b1193f").unwrap();
        assert!(
            opened.contains_key(&deal_id),
            "Deal should be correctly parsed and added to state"
        );
    }
}

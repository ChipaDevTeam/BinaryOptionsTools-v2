use binary_options_tools::pocketoption::state::TradeState;
use binary_options_tools::pocketoption::types::Deal;
use std::sync::Arc;

#[tokio::test]
async fn test_trade_state_transition_atomicity() {
    let trade_state = Arc::new(TradeState::default());

    // We need a valid Deal JSON to deserialize
    let deal_json = r#"{
        "id": "550e8400-e29b-41d4-a716-446655440000",
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
    }"#;

    let deal: Deal = serde_json::from_str(deal_json).expect("Failed to deserialize mock deal");
    let deal_id = deal.id;

    // Start with the deal in opened_deals
    trade_state.add_opened_deal(deal.clone()).await;

    let trade_state_clone = trade_state.clone();
    let move_handle = tokio::spawn(async move {
        // Simulating a delay to make the race more likely (although join! should prevent it)
        let deals = vec![deal.clone()];
        trade_state_clone.update_closed_deals(deals).await;
    });

    let trade_state_clone2 = trade_state.clone();
    let check_handle = tokio::spawn(async move {
        for _ in 0..500 {
            let is_opened = trade_state_clone2.contains_opened_deal(deal_id).await;
            let is_closed = trade_state_clone2.contains_closed_deal(deal_id).await;

            // Atomicity check: The deal should NOT be missing from both.
            // If it is missing from both, the transition is NOT atomic for the reader.
            assert!(
                is_opened || is_closed,
                "Deal {} vanished during transition! State must be atomic.",
                deal_id
            );

            // Consistency check: The deal should NOT be in both.
            assert!(
                !(is_opened && is_closed),
                "Deal {} is in both states! State must be consistent.",
                deal_id
            );

            tokio::task::yield_now().await;
        }
    });

    move_handle.await.unwrap();
    check_handle.await.unwrap();
}

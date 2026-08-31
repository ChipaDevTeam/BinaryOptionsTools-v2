use binary_options_tools::closeoption::{CloseOption, StateBuilder};

#[tokio::test]
async fn test_state_builder() {
    let state = StateBuilder::new()
        .token("test_token")
        .sid("test_sid")
        .public_code("pub_code")
        .hidden_code("hid_code")
        .demo(true)
        .build()
        .unwrap();

    assert_eq!(state.token, "test_token");
    assert_eq!(state.sid, "test_sid");
    assert_eq!(state.public_code, "pub_code");
    assert_eq!(state.hidden_code, "hid_code");
    assert!(state.is_demo);
    assert_eq!(state.acc_type(), "demo");
}

#[tokio::test]
async fn test_state_builder_real_account() {
    let state = StateBuilder::new()
        .token("test_token")
        .sid("test_sid")
        .public_code("pub_code")
        .hidden_code("hid_code")
        .demo(false)
        .build()
        .unwrap();

    assert_eq!(state.acc_type(), "real");
}

#[tokio::test]
async fn test_ws_url() {
    let state = StateBuilder::new()
        .token("test_token")
        .sid("abc123")
        .public_code("pub_code")
        .hidden_code("hid_code")
        .build()
        .unwrap();

    let url = state.ws_url();
    assert!(url.contains("sid=abc123"));
    assert!(url.contains("EIO=3"));
    assert!(url.contains("transport=websocket"));
}

#[tokio::test]
async fn test_state_builder_missing_fields() {
    // Missing token
    let result = StateBuilder::new()
        .sid("test_sid")
        .public_code("pub")
        .hidden_code("hid")
        .build();
    assert!(result.is_err());

    // Missing sid
    let result = StateBuilder::new()
        .token("test_token")
        .public_code("pub")
        .hidden_code("hid")
        .build();
    assert!(result.is_err());

    // Missing public_code
    let result = StateBuilder::new()
        .token("test_token")
        .sid("test_sid")
        .hidden_code("hid")
        .build();
    assert!(result.is_err());

    // Missing hidden_code
    let result = StateBuilder::new()
        .token("test_token")
        .sid("test_sid")
        .public_code("pub")
        .build();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_clear_temporal_data() {
    let state = StateBuilder::new()
        .token("test_token")
        .sid("test_sid")
        .public_code("pub_code")
        .hidden_code("hid_code")
        .build()
        .unwrap();

    // Set some data
    state.update_balance(100.0).await;
    let expected_offset = chrono::Utc::now().timestamp() + 3600;
    state.update_server_time_offset(expected_offset).await;

    // Verify data is set
    assert_eq!(state.get_balance().await, Some(100.0));
    assert!((state.get_server_time_offset().await - 3600).abs() < 5);

    // Clear temporal data
    state.clear_temporal_data().await;

    // Verify data is cleared
    assert_eq!(state.get_balance().await, None);
    assert_eq!(state.get_server_time_offset().await, 0);
}

#[tokio::test]
async fn test_asset_updates() {
    use binary_options_tools::closeoption::types::{PriceData, AssetPrice};
    use std::collections::HashMap;

    let state = StateBuilder::new()
        .token("test_token")
        .sid("test_sid")
        .public_code("pub_code")
        .hidden_code("hid_code")
        .build()
        .unwrap();

    let mut prices = HashMap::new();
    prices.insert("EURUSD".to_string(), AssetPrice { bid: 1.1000, ask: 1.1002, main: 1.1001 });
    prices.insert("GBPUSD".to_string(), AssetPrice { bid: 1.3000, ask: 1.3002, main: 1.3001 });

    let price_data = PriceData {
        prices,
        timestamp: 1704067200,
    };

    state.update_assets(&price_data).await;

    let assets = state.get_assets().await;
    assert_eq!(assets.len(), 2);
    assert!(assets.contains_key("EURUSD"));
    assert!(assets.contains_key("GBPUSD"));

    let eurusd = assets.get("EURUSD").unwrap();
    assert_eq!(eurusd.bid, 1.1000);
    assert_eq!(eurusd.ask, 1.1002);
    assert_eq!(eurusd.main, 1.1001);
}

#[tokio::test]
async fn test_get_ticks_request_shape() {
    use binary_options_tools::closeoption::types::Get30MinRequest;

    let state = StateBuilder::new()
        .token("test_token")
        .sid("test_sid")
        .public_code("pub_code")
        .hidden_code("hid_code")
        .demo(true)
        .build()
        .unwrap();

    let request = Get30MinRequest {
        token: state.token.clone(),
        ps_type: "30min".to_string(),
        public_code: state.public_code.clone(),
        hidden_code: state.hidden_code.clone(),
        acc_type: state.acc_type().to_string(),
        pair: "EUR/USD:AFX".to_string(),
        contest_type: "".to_string(),
    };

    assert_eq!(request.token, "test_token");
    assert_eq!(request.ps_type, "30min");
    assert_eq!(request.public_code, "pub_code");
    assert_eq!(request.hidden_code, "hid_code");
    assert_eq!(request.acc_type, "demo");
    assert_eq!(request.pair, "EUR/USD:AFX");
    assert_eq!(request.contest_type, "");
}

#[tokio::test]
async fn test_get_ticks_result_parsing() {
    use binary_options_tools::closeoption::types::{Get30MinResult, Candle};

    let json = r#"{
        "candles": [
            {"timeStamp": 1704067200, "value": 1.1001},
            {"timeStamp": 1704067201, "value": 1.1002}
        ],
        "pair": "EUR/USD:AFX"
    }"#;

    let result: Get30MinResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.candles.len(), 2);
    assert_eq!(result.candles[0].time_stamp, 1704067200);
    assert_eq!(result.candles[0].value, 1.1001);
    assert_eq!(result.candles[1].time_stamp, 1704067201);
    assert_eq!(result.candles[1].value, 1.1002);
    assert_eq!(result.pair, "EUR/USD:AFX");
}

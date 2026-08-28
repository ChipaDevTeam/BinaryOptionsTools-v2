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
    state.update_server_time_offset(3600).await;

    // Verify data is set
    assert_eq!(state.get_balance().await, Some(100.0));
    assert_eq!(state.get_server_time_offset().await, 3600);

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
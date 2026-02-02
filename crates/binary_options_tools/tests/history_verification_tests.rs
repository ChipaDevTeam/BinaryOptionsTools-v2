use std::sync::Arc;
use uuid::Uuid;
use binary_options_tools_core_pre::reimports::{Message, bounded_async};
use binary_options_tools_core_pre::traits::ApiModule;
use binary_options_tools::pocketoption::modules::historical_data::{HistoricalDataApiModule, Command, CommandResponse};
use binary_options_tools::pocketoption::ssid::Ssid;
use binary_options_tools::pocketoption::state::StateBuilder;

#[tokio::test]
async fn test_history_validation_and_compilation() {
    // Setup channels
    let (cmd_tx, cmd_rx) = bounded_async(10);
    let (resp_tx, resp_rx) = bounded_async(10);
    let (msg_tx, msg_rx) = bounded_async(10);
    let (ws_tx, ws_rx) = bounded_async(10);

    // Create shared state
    let dummy_ssid_str = r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
    let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");
    let state = Arc::new(
        StateBuilder::default()
            .ssid(ssid)
            .build()
            .expect("Failed to build state"),
    );

    // Initialize the module
    let mut module = HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

    // Spawn the module loop
    tokio::spawn(async move {
        if let Err(e) = module.run().await {
            eprintln!("Module run error: {:?}", e);
        }
    });

    // a. Request for candles of AUDCAD_otc with period 60
    let req_id = Uuid::new_v4();
    let asset = "AUDCAD_otc".to_string();
    let period = 60;

    cmd_tx.send(Command::GetCandles {
        asset: asset.clone(),
        period,
        req_id,
    }).await.expect("Failed to send command");

    // Consume the changeSymbol message to avoid blocking
    let _ = ws_rx.recv().await.expect("Failed to receive changeSymbol message");

    // b. Receiving an updateHistoryNewFast message for EURUSD_otc with period 1 (should be ignored)
    let ignored_payload = r#"{
        "asset": "EURUSD_otc",
        "period": 1,
        "history": [[1769988863.979, 1.18206]],
        "candles": []
    }"#;
    msg_tx.send(Arc::new(Message::Text(ignored_payload.to_string().into()))).await.expect("Failed to send ignored message");

    // c. Receiving an updateHistoryNewFast message for AUDCAD_otc with period 60
    // Verify that if it only contains history (ticks) and no candles, it compiles them.
    // We'll provide ticks that span across a minute boundary to see if it compiles at least one candle.
    let accepted_payload = r#"{
        "asset": "AUDCAD_otc",
        "period": 60,
        "history": [
            [1769988869.465, 0.89163],
            [1769988870.000, 0.89170],
            [1769988929.999, 0.89180]
        ],
        "candles": []
    }"#;
    msg_tx.send(Arc::new(Message::Text(accepted_payload.to_string().into()))).await.expect("Failed to send accepted message");

    // Verify the response
    let response = resp_rx.recv().await.expect("Failed to receive module response");

    match response {
        CommandResponse::Candles { req_id: r_id, candles } => {
            assert_eq!(r_id, req_id);
            // The ticks are in the same 60s bucket (1769988840 to 1769988899) and (1769988900 to 1769988959)
            // 1769988869.465 -> bucket 1769988840
            // 1769988870.000 -> bucket 1769988840
            // 1769988929.999 -> bucket 1769988900
            assert_eq!(candles.len(), 2, "Should have compiled 2 candles from ticks");
            
            // Check first candle (bucket 1769988840)
            assert_eq!(candles[0].timestamp, 1769988840.0);
            
            // Check second candle (bucket 1769988900)
            assert_eq!(candles[1].timestamp, 1769988900.0);
        }
        _ => panic!("Expected Candles response"),
    }
}

#[tokio::test]
async fn test_candle_format_ochlv() {
    // Setup channels
    let (cmd_tx, cmd_rx) = bounded_async(10);
    let (resp_tx, resp_rx) = bounded_async(10);
    let (msg_tx, msg_rx) = bounded_async(10);
    let (ws_tx, ws_rx) = bounded_async(10);

    // Create shared state
    let dummy_ssid_str = r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
    let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");
    let state = Arc::new(
        StateBuilder::default()
            .ssid(ssid)
            .build()
            .expect("Failed to build state"),
    );

    // Initialize the module
    let mut module = HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

    // Spawn the module loop
    tokio::spawn(async move {
        if let Err(e) = module.run().await {
            eprintln!("Module run error: {:?}", e);
        }
    });

    // Request candles
    let asset = "AUDCAD_otc".to_string();
    let period = 60;
    cmd_tx.send(Command::GetCandles {
        asset: asset.clone(),
        period,
        req_id: Uuid::new_v4(),
    }).await.unwrap();

    // Consume changeSymbol
    let _ = ws_rx.recv().await.unwrap();

    // Send payload with OCHLV data
    // Format: [timestamp, open, close, high, low, volume]
    // 1769988660, 0.89232 (O), 0.89176 (C), 0.89271 (H), 0.89149 (L), 110 (V)
    let payload = r#"{
        "asset": "AUDCAD_otc",
        "period": 60,
        "candles": [
            [1769988660, 0.89232, 0.89176, 0.89271, 0.89149, 110]
        ]
    }"#;
    msg_tx.send(Arc::new(Message::Text(payload.to_string().into()))).await.unwrap();

    // Verify response
    let response = resp_rx.recv().await.unwrap();
    if let CommandResponse::Candles { candles, .. } = response {
        assert_eq!(candles.len(), 1);
        let c = &candles[0];
        assert_eq!(c.timestamp, 1769988660.0);
        assert_eq!(c.open.to_string(), "0.89232");
        assert_eq!(c.close.to_string(), "0.89176");
        assert_eq!(c.high.to_string(), "0.89271");
        assert_eq!(c.low.to_string(), "0.89149");
        assert_eq!(c.volume.unwrap().to_string(), "110");
    } else {
        panic!("Expected Candles response");
    }
}

#[tokio::test]
async fn test_ticks_request() {
    // Setup channels
    let (cmd_tx, cmd_rx) = bounded_async(10);
    let (resp_tx, resp_rx) = bounded_async(10);
    let (msg_tx, msg_rx) = bounded_async(10);
    let (ws_tx, ws_rx) = bounded_async(10);

    // Create shared state
    let dummy_ssid_str = r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
    let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");
    let state = Arc::new(
        StateBuilder::default()
            .ssid(ssid)
            .build()
            .expect("Failed to build state"),
    );

    // Initialize the module
    let mut module = HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

    // Spawn the module loop
    tokio::spawn(async move {
        if let Err(e) = module.run().await {
            eprintln!("Module run error: {:?}", e);
        }
    });

    // Request ticks
    let req_id = Uuid::new_v4();
    let asset = "EURUSD_otc".to_string();
    let period = 1;

    cmd_tx.send(Command::GetTicks {
        asset: asset.clone(),
        period,
        req_id,
    }).await.expect("Failed to send command");

    // Consume changeSymbol
    let _ = ws_rx.recv().await.unwrap();

    // Send payload with ticks
    let payload = r#"{
        "asset": "EURUSD_otc",
        "period": 1,
        "history": [
            [1769988869.465, 1.18206],
            [1769988870.000, 1.18210]
        ]
    }"#;
    msg_tx.send(Arc::new(Message::Text(payload.to_string().into()))).await.expect("Failed to send message");

    // Verify response
    let response = resp_rx.recv().await.expect("Failed to receive module response");
    if let CommandResponse::Ticks { req_id: r_id, ticks } = response {
        assert_eq!(r_id, req_id);
        assert_eq!(ticks.len(), 2);
        assert_eq!(ticks[0], (1769988869.465, 1.18206));
        assert_eq!(ticks[1], (1769988870.000, 1.18210));
    } else {
        panic!("Expected Ticks response");
    }
}

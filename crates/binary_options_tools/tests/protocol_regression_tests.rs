//! End-to-end regression tests against an in-process fake PocketOption broker.
//!
//! The frames below mirror a capture of the live server (`websocket_export.csv`):
//! the asset list is pushed to every socket, `successauth` is announced with a
//! two-step frame whose binary half is only `{"id": ...}`, and `changeSymbol` is
//! what makes the server start answering history requests.
//!
//! These tests exist because the client used to look for `"serverName"` in the
//! binary half of `successauth`, a key the server no longer sends. Authentication
//! was therefore never detected, the initial snapshot (`changeSymbol`, `getBalance`,
//! `subfor`, …) was never requested, and every candle/tick request blocked until it
//! timed out.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use binary_options_tools::pocketoption::PocketOption;
use futures_util::{SinkExt, StreamExt};
use rust_decimal_macros::dec;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// One active asset, shaped like the real `updateAssets` payload.
const ASSETS: &str = r#"[[1,"EURUSD_otc","Euro/US Dollar OTC","currency",3,92,60,30,3,1,0,5,[],1791936000,true,[{"time":1},{"time":5},{"time":60}],-1,60,-1]]"#;

/// Auth payload for a demo account; only its shape matters to the fake broker.
const DEMO_SSID: &str =
    r#"42["auth",{"session":"dummy_session_id","isDemo":1,"uid":12345678,"platform":2}]"#;

/// Starts a fake broker and returns its WebSocket URL plus the frames it received.
async fn spawn_fake_broker() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

        ws.send(Message::text(
            r#"0{"sid":"engine","upgrades":[],"pingInterval":25000,"pingTimeout":20000}"#,
        ))
        .await
        .unwrap();

        // The real server only answers history requests for the symbol that is
        // currently selected, so nothing is answered until `changeSymbol` arrives.
        let mut symbol_selected = false;

        while let Some(Ok(msg)) = ws.next().await {
            let Message::Text(text) = msg else { break };
            let text = text.to_string();
            received_clone.lock().unwrap().push(text.clone());

            if text == "40" {
                ws.send(Message::text(r#"40{"sid":"socket"}"#))
                    .await
                    .unwrap();
                continue;
            }

            if text.starts_with(r#"42["auth""#) {
                // Assets are pushed to every socket, authenticated or not.
                send_two_step(&mut ws, "updateAssets", ASSETS).await;
                // `successauth` confirmation: the binary half carries only the id.
                send_two_step(&mut ws, "successauth", r#"{"id":"socket"}"#).await;
                send_two_step(
                    &mut ws,
                    "successupdateBalance",
                    r#"{"isDemo":1,"balance":33406.18}"#,
                )
                .await;
                continue;
            }

            if text.starts_with(r#"42["changeSymbol""#) {
                symbol_selected = true;
                // Streaming history only starts once a symbol is selected.
                send_two_step(
                    &mut ws,
                    "updateHistoryNewFast",
                    r#"{"asset":"EURUSD_otc","period":60,"history":[[1791932074.209,1.78135],[1791932075.21,1.78146]]}"#,
                )
                .await;
                continue;
            }

            if text.starts_with(r#"42["loadHistoryPeriod""#) {
                if !symbol_selected {
                    // Mirrors the live server: no answer until a symbol is selected.
                    continue;
                }
                let request: serde_json::Value = serde_json::from_str(&text["42".len()..]).unwrap();
                let index = request[1]["index"].as_u64().unwrap();
                let payload = serde_json::json!({
                    "asset": "EURUSD_otc",
                    "index": index,
                    "data": [
                        {"symbol_id": 1, "time": 1791931300, "open": 1.79434, "close": 1.79427, "high": 1.79447, "low": 1.79427, "volume": 7},
                        {"symbol_id": 1, "time": 1791931305, "open": 1.79431, "close": 1.79453, "high": 1.79473, "low": 1.79406, "volume": 7},
                        {"symbol_id": 1, "time": 1791931310, "open": 1.79466, "close": 1.79473, "high": 1.7948, "low": 1.79464, "volume": 6}
                    ]
                });
                send_two_step(&mut ws, "loadHistoryPeriodFast", &payload.to_string()).await;
                continue;
            }

            if text == "2" {
                ws.send(Message::text("3")).await.unwrap();
            }
        }
    });

    (
        format!("ws://127.0.0.1:{port}/socket.io/?EIO=4&transport=websocket"),
        received,
    )
}

/// Sends the two-step shape the server uses: a header announcing a binary attachment,
/// followed by the payload itself.
async fn send_two_step<S>(ws: &mut S, event: &str, payload: &str)
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    ws.send(Message::text(format!(
        r#"451-["{event}",{{"_placeholder":true,"num":0}}]"#
    )))
    .await
    .unwrap();
    ws.send(Message::Binary(payload.as_bytes().to_vec().into()))
        .await
        .unwrap();
}

#[tokio::test]
async fn authentication_is_detected_from_the_two_step_successauth() {
    let (url, _received) = spawn_fake_broker().await;
    let api = PocketOption::new_with_url(DEMO_SSID, url).await.unwrap();

    // The balance frame is only forwarded to a module matching `successupdateBalance`,
    // which in turn only happens once the auth was recognised.
    let balance = api.balance().await;
    assert_eq!(balance, dec!(33406.18));

    let assets = api.assets().await.expect("assets must be loaded");
    assert!(assets.get("EURUSD_otc").is_some());
}

#[tokio::test]
async fn initial_snapshot_is_requested_after_auth() {
    let (url, received) = spawn_fake_broker().await;
    let api = PocketOption::new_with_url(DEMO_SSID, url).await.unwrap();

    // Give the module a moment to flush the snapshot, then assert on the wire.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let sent = received.lock().unwrap().clone();
            if sent.iter().any(|m| m.contains(r#"42["getBalance"]"#))
                && sent.iter().any(|m| m.starts_with(r#"42["changeSymbol""#))
                && sent.iter().any(|m| m.starts_with(r#"42["subfor""#))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("the initial snapshot must be requested once auth is confirmed");

    api.shutdown().await.ok();
}

#[tokio::test]
async fn candles_are_returned_for_the_selected_symbol() {
    let (url, _received) = spawn_fake_broker().await;
    let api = PocketOption::new_with_url(DEMO_SSID, url).await.unwrap();

    let candles =
        tokio::time::timeout(Duration::from_secs(10), api.get_candles("EURUSD_otc", 5, 3))
            .await
            .expect("candles must not hang: the symbol is selected and the server answers")
            .expect("candles request must succeed");

    assert_eq!(candles.len(), 3, "candles: {candles:?}");
    assert_eq!(candles[0].open, dec!(1.79434));
    assert_eq!(candles[2].close, dec!(1.79473));
    assert_eq!(candles[0].symbol, "EURUSD_otc");
}

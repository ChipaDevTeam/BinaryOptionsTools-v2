use super::common::*;
use crate::pocketoption::types::Action;
use binary_options_tools_core::reimports::Message;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_concurrent_identical_trades_hammer() {
    let setup = create_test_setup().await;
    let asset = "EURUSD_otc".to_string();
    let amount = dec!(10.0);
    let time = 60;

    // Place 5 identical trades
    let mut handles = Vec::new();
    for _ in 0..5 {
        let h = setup.handle.clone();
        let a = asset.clone();
        handles.push(tokio::spawn(async move {
            h.trade(a, Action::Call, amount, time).await
        }));
    }

    // Wait for all 5 to be sent to WS
    let mut req_ids = Vec::new();
    for _ in 0..5 {
        if let Ok(Message::Text(text)) = timeout(Duration::from_secs(1), setup.ws_rx.recv())
            .await
            .unwrap()
        {
            // 42["openOrder",{"amount":"10.0","asset":"EURUSD_otc","action":"call","isDemo":0,"requestId":"...","optionType":100,"time":60}]
            let start = text.find('{').unwrap();
            let end = text.rfind('}').unwrap();
            let json_str = &text[start..end + 1];
            let v: serde_json::Value = serde_json::from_str(json_str).unwrap();
            let req_id = Uuid::parse_str(v["requestId"].as_str().unwrap()).unwrap();
            req_ids.push(req_id);
        }
    }

    assert_eq!(req_ids.len(), 5);

    // Now simulate responses from server.
    // Server doesn't send requestId for failures, it uses asset/amount matching.
    // For successes, it DOES send requestId.

    // 1. Success for 2nd request
    let deal2 = create_test_deal(req_ids[1], &asset);
    let resp2 = format!(
        r#"42["successopenOrder",{}]"#,
        serde_json::to_string(&deal2).unwrap()
    );
    setup
        .msg_tx
        .send(Arc::new(Message::Text(resp2.into())))
        .await
        .unwrap();

    // 2. Failure (will be matched to 1st request because it's the oldest in failure_matching queue)
    let fail_data = create_test_fail(&asset, amount);
    let resp_fail = format!(
        r#"42["failopenOrder",{}]"#,
        serde_json::to_string(&fail_data).unwrap()
    );
    setup
        .msg_tx
        .send(Arc::new(Message::Text(resp_fail.into())))
        .await
        .unwrap();

    // 3. Success for 4th request
    let deal4 = create_test_deal(req_ids[3], &asset);
    let resp4 = format!(
        r#"42["successopenOrder",{}]"#,
        serde_json::to_string(&deal4).unwrap()
    );
    setup
        .msg_tx
        .send(Arc::new(Message::Text(resp4.into())))
        .await
        .unwrap();

    // Now collect results from handles
    // handle 2 (req_ids[1]) should be Success
    // handle 1 (req_ids[0]) should be Fail
    // handle 4 (req_ids[3]) should be Success
    // handle 3 and 5 are still pending (we can ignore or fail them later)

    // Note: Since we spawned them, we don't easily know which JoinHandle corresponds to which req_id
    // unless we mapped them. But the TradesApiModule routes them based on req_id for success.
    // For failure, it routes to the OLDEST one in its internal queue.

    // Let's wait for the ones we expect to finish
    // Since they are in tokio::spawn, we just await them.
    // Actually, req_ids[1] was deal2, so handles[1] should return Ok(deal2).
    // req_ids[0] was matched to fail_data, so handles[0] should return Err(FailOpenOrder).
    // req_ids[3] was deal4, so handles[3] should return Ok(deal4).

    let res1 = handles.remove(0).await.unwrap();
    let res2 = handles.remove(0).await.unwrap(); // handles[1] is now at index 0
    let _res3 = handles.remove(0); // skip handles[2]
    let res4 = handles.remove(0).await.unwrap(); // handles[3] is now at index 0

    assert!(res1.is_err());
    assert!(res2.is_ok());
    assert!(res4.is_ok());

    if let Err(e) = res1 {
        assert!(format!("{:?}", e).contains("Insufficient balance"));
    }
}

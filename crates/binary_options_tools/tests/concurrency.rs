use binary_options_tools::pocketoption::error::PocketError;
use binary_options_tools::pocketoption::types::{Action, RequestId};
use binary_options_tools_core::reimports::Message;
use rust_decimal_macros::dec;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

mod common {
    include!("common/mod.rs");
}

use common::{create_test_deal, create_test_fail, create_test_setup};

/// Upper bound for every wait in this test. A regression should surface as a
/// failure with a message, never as a test run that hangs.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn test_concurrent_identical_trades_hammer() {
    let setup = create_test_setup().await;
    let asset = "EURUSD_otc".to_string();
    let amount = dec!(10.0);
    let time = 60;

    // Five indistinguishable trades are placed at once. Each one gets an
    // explicit request id: `TradesHandle::trade` invents its own, and the order
    // in which the spawned tasks reach the module is not the order in which
    // they were spawned, so the ids are the only reliable way to tell the
    // trades apart.
    let req_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

    let (done_tx, mut done_rx) = unbounded_channel();
    let mut tasks = Vec::new();
    for req_id in req_ids.iter().copied() {
        let handle = setup.handle.clone();
        let asset = asset.clone();
        let done_tx = done_tx.clone();
        tasks.push(tokio::spawn(async move {
            let result = handle
                .trade_with_id(asset, Action::Call, amount, time, req_id)
                .await;
            let _ = done_tx.send((req_id, result));
        }));
    }
    drop(done_tx);

    // Every order must reach the socket carrying one of our request ids.
    // Arrival order is deliberately not asserted.
    let mut sent = HashSet::new();
    for _ in 0..5 {
        let msg = timeout(TEST_TIMEOUT, setup.ws_rx.recv())
            .await
            .expect("order was not sent to the socket")
            .expect("websocket channel closed");
        let text = match msg {
            Message::Text(text) => text,
            _ => panic!("orders must be sent as text frames"),
        };
        let start = text.find('{').expect("order frame has no JSON body");
        let end = text.rfind('}').expect("order frame has no JSON body");
        let value: serde_json::Value = serde_json::from_str(&text[start..end + 1]).unwrap();
        let req_id = value["requestId"]
            .as_str()
            .expect("order frame has no requestId");
        sent.insert(Uuid::parse_str(req_id).unwrap());
    }
    assert_eq!(
        sent,
        req_ids.iter().copied().collect::<HashSet<_>>(),
        "the socket saw a different set of request ids than the five trades"
    );

    // `failopenOrder` carries no request id, so the module can only attribute
    // it to one of the still-pending orders, and which one is not part of the
    // contract. Send the failure first and observe who it hit, then answer two
    // of the survivors by id. Answering a trade the failure already took would
    // resolve nothing (the module logs it as an unknown request id), so fixing
    // the victim up front is what makes this test deterministic.
    let fail = create_test_fail(&asset, amount);
    setup
        .msg_tx
        .send(Arc::new(Message::Text(
            format!(
                r#"42["failopenOrder",{}]"#,
                serde_json::to_string(&fail).unwrap()
            )
            .into(),
        )))
        .await
        .unwrap();

    let (rejected_id, rejected) = timeout(TEST_TIMEOUT, done_rx.recv())
        .await
        .expect("the failure reply was not routed to any pending trade")
        .expect("trade task channel closed");
    assert!(
        req_ids.contains(&rejected_id),
        "a trade that was never placed was rejected: {rejected_id}"
    );
    let error = rejected.expect_err("the reject reply resolved a trade as a success");
    assert!(
        matches!(error, PocketError::FailOpenOrder { .. }),
        "unexpected error for a rejected order: {error}"
    );

    // Pick two of the trades the failure did not take and answer them by id.
    let answered: Vec<Uuid> = req_ids
        .iter()
        .copied()
        .filter(|id| *id != rejected_id)
        .take(2)
        .collect();
    let deals: Vec<_> = answered
        .iter()
        .map(|id| create_test_deal(*id, &asset))
        .collect();
    let expected_deal_ids: HashSet<Uuid> = deals.iter().map(|deal| deal.id).collect();

    for deal in &deals {
        setup
            .msg_tx
            .send(Arc::new(Message::Text(
                format!(
                    r#"42["successopenOrder",{}]"#,
                    serde_json::to_string(deal).unwrap()
                )
                .into(),
            )))
            .await
            .unwrap();
    }

    // Only three of the five orders were answered; the other two stay pending
    // on purpose.
    let mut succeeded = HashSet::new();
    for _ in 0..deals.len() {
        let (req_id, result) = timeout(TEST_TIMEOUT, done_rx.recv())
            .await
            .expect("a success reply did not resolve its trade")
            .expect("trade task channel closed");
        assert!(
            answered.contains(&req_id),
            "trade {req_id} resolved, but only {answered:?} were answered"
        );
        let deal = result.expect("an answered trade reported an error");
        assert_eq!(
            Some(RequestId::Uuid(req_id)),
            deal.request_id,
            "trade {req_id} was resolved with another trade's deal"
        );
        succeeded.insert(deal.id);
    }

    assert_eq!(
        succeeded, expected_deal_ids,
        "the successes were not routed to the deals that named them"
    );

    for task in tasks {
        task.abort();
    }
}

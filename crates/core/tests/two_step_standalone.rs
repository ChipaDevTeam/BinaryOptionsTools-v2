/// Standalone test to verify Rule macro DSL for two-step protocol
/// Run with: cargo test --test two_step_standalone -- --nocapture
use binary_options_tools_core::{traits::Rule, Rule};
use tokio_tungstenite::tungstenite::Message;

// Define the rule using the DSL macro
#[Rule]
#[rule({
    contains(r#"["successopenOrder","#) -> message_type(Binary)
})]
struct SuccessOpenOrderRule;

#[Rule]
#[rule({
    contains(r#"["failopenOrder","#) -> message_type(Binary)
})]
struct FailOpenOrderRule;

#[Rule]
#[rule({
    (contains(r#"["successopenOrder","#)
    | contains(r#"["failopenOrder","#)
    ) -> message_type(Binary)
})]
struct CombinedTradeRule;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_open_order_basic_sequence() {
        println!("\n=== Test: Basic Two-Step Sequence ===");
        let rule = SuccessOpenOrderRule::new();

        // Step 1: Text header with placeholder
        let header =
            Message::text(r#"451-["successopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        println!("Sending header: {:?}", header);
        let result = rule.call(&header);
        println!("Header result: {} (expected: false)", result);
        assert!(!result, "Header should return false and set state");

        // Step 2: Binary body
        let body = Message::binary(vec![0x01, 0x02, 0x03]);
        println!("Sending binary body: {:?}", body);
        let result = rule.call(&body);
        println!("Binary result: {} (expected: true)", result);
        assert!(result, "Binary should return true after header");

        // Step 3: Orphan binary (no preceding header)
        let orphan = Message::binary(vec![0x04, 0x05]);
        println!("Sending orphan binary: {:?}", orphan);
        let result = rule.call(&orphan);
        println!("Orphan result: {} (expected: false)", result);
        assert!(!result, "Orphan binary should return false");

        println!("✓ Test passed!\n");
    }

    #[test]
    fn test_fail_open_order() {
        println!("\n=== Test: Fail Open Order ===");
        let rule = FailOpenOrderRule::new();

        let header =
            Message::text(r#"451-["failopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        println!("Sending fail header: {:?}", header);
        assert!(!rule.call(&header), "Fail header should return false");

        let body = Message::binary(vec![0xFF, 0xEE]);
        println!("Sending fail body: {:?}", body);
        assert!(rule.call(&body), "Fail binary should return true");

        println!("✓ Test passed!\n");
    }

    #[test]
    fn test_combined_rule() {
        println!("\n=== Test: Combined Success/Fail Rule ===");
        let rule = CombinedTradeRule::new();

        // Test successopenOrder
        println!("Testing successopenOrder...");
        let success_header =
            Message::text(r#"451-["successopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        assert!(!rule.call(&success_header));

        let success_body = Message::binary(b"success_data".to_vec());
        assert!(rule.call(&success_body));

        // Test failopenOrder
        println!("Testing failopenOrder...");
        let fail_header =
            Message::text(r#"451-["failopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        assert!(!rule.call(&fail_header));

        let fail_body = Message::binary(b"fail_data".to_vec());
        assert!(rule.call(&fail_body));

        println!("✓ Test passed!\n");
    }

    #[test]
    fn test_wrong_event_name() {
        println!("\n=== Test: Wrong Event Name ===");
        let rule = SuccessOpenOrderRule::new();

        let wrong_header =
            Message::text(r#"451-["wrongEventName",{"_placeholder":true,"num":0}]"#.to_string());
        println!("Sending wrong event: {:?}", wrong_header);
        assert!(
            !rule.call(&wrong_header),
            "Wrong event should not match"
        );

        let body = Message::binary(vec![0x01, 0x02]);
        println!("Sending binary after wrong event: {:?}", body);
        assert!(
            !rule.call(&body),
            "Binary should not pass without matching header"
        );

        println!("✓ Test passed!\n");
    }

    #[test]
    fn test_reset_clears_state() {
        println!("\n=== Test: Reset Functionality ===");
        let rule = SuccessOpenOrderRule::new();

        // Set up state with header
        let header =
            Message::text(r#"451-["successopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        println!("Sending header: {:?}", header);
        assert!(!rule.call(&header));

        // Reset should clear state
        println!("Calling reset()...");
        rule.reset();

        // Binary should not pass after reset
        let body = Message::binary(vec![0x01, 0x02]);
        println!("Sending binary after reset: {:?}", body);
        let result = rule.call(&body);
        println!("Binary after reset result: {} (expected: false)", result);
        assert!(!result, "Binary should not pass after reset");

        println!("✓ Test passed!\n");
    }

    #[test]
    fn test_multiple_pairs() {
        println!("\n=== Test: Multiple Sequential Pairs ===");
        let rule = CombinedTradeRule::new();

        // Pair 1
        println!("Pair 1: successopenOrder");
        let h1 =
            Message::text(r#"451-["successopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        let b1 = Message::binary(b"data1".to_vec());
        assert!(!rule.call(&h1));
        assert!(rule.call(&b1));

        // Pair 2
        println!("Pair 2: failopenOrder");
        let h2 =
            Message::text(r#"451-["failopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        let b2 = Message::binary(b"data2".to_vec());
        assert!(!rule.call(&h2));
        assert!(rule.call(&b2));

        // Pair 3
        println!("Pair 3: successopenOrder again");
        let h3 =
            Message::text(r#"451-["successopenOrder",{"_placeholder":true,"num":0}]"#.to_string());
        let b3 = Message::binary(b"data3".to_vec());
        assert!(!rule.call(&h3));
        assert!(rule.call(&b3));

        println!("✓ Test passed!\n");
    }
}

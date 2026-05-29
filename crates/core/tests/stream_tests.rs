use binary_options_tools_core::reimports::{bounded_async, Message};
use binary_options_tools_core::utils::stream::FilteredRecieverStream;
use binary_options_tools_core::traits::Rule;
use binary_options_tools_core::rules::RuleBuilder;
use binary_options_tools_core::error::CoreResult;
use futures_util::StreamExt;

#[tokio::test]
async fn test_filtered_receiver_stream_respects_filter_no_timeout() {
    let (tx, rx) = bounded_async::<Message>(10);
    
    // Create a filter that only accepts messages containing "match"
    struct MatchFilter;
    impl Rule for MatchFilter {
        fn call(&self, msg: &Message) -> bool {
            match msg {
                Message::Text(text) => text.contains("match"),
                _ => false,
            }
        }
        fn reset(&self) {}
    }
    
    let stream_obj = FilteredRecieverStream::new_filtered(rx, Box::new(MatchFilter));
    let mut stream = stream_obj.to_stream();
    
    // Send a non-matching message
    tx.send(Message::Text("ignore me".into())).await.unwrap();
    // Send a matching message
    tx.send(Message::Text("this is a match".into())).await.unwrap();
    
    // Receive from stream - should skip "ignore me" and get "this is a match"
    let msg_res: Option<CoreResult<Message>> = stream.next().await;
    let msg = msg_res.unwrap().unwrap();
    
    if let Message::Text(text) = msg {
        assert_eq!(text.as_str(), "this is a match");
    } else {
        panic!("Expected text message");
    }
}

#[tokio::test]
async fn test_filtered_receiver_stream_with_timeout() {
    let (_tx, rx) = bounded_async::<Message>(10);
    // Note: FilteredRecieverStream doesn't have a public way to get default_filter() 
    // but we can use a closure.
    let filter = Box::new(|_: &Message| true);
    let stream_obj = FilteredRecieverStream::new(rx, Some(std::time::Duration::from_millis(10)), filter);
    let mut stream = stream_obj.to_stream();
    
    let result: Option<CoreResult<Message>> = stream.next().await;
    assert!(result.unwrap().is_err());
}

#[test]
fn test_regex_rule_correctness() {
    let rule = RuleBuilder::text_regex("^[0-9]+$").build();
    
    let msg1 = Message::Text("12345".into());
    let msg2 = Message::Text("123a45".into());
    let msg3 = Message::Text("abc".into());
    
    assert!(rule.call(&msg1));
    assert!(!rule.call(&msg2));
    assert!(!rule.call(&msg3));
}

//! Integration tests for the Raw module functionality

use binary_options_tools::pocketoption::modules::raw::{
    Command, CommandResponse, Outgoing, RawApiModule,
};
use binary_options_tools::validator::Validator;
use binary_options_tools_core_pre::reimports::{AsyncReceiver, AsyncSender};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tracing_test::traced_test; // Add this import
use uuid::Uuid; // Import from reimports

#[cfg(test)]
mod tests {
    use super::*;
    use binary_options_tools::traits::ValidatorTrait;
    use binary_options_tools_core_pre::reimports::bounded_async;
    use binary_options_tools_core_pre::traits::{ApiModule, Rule};
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    // Mock state for testing
    #[derive(Debug)]
    struct MockState {
        pub raw_validators: RwLock<HashMap<Uuid, Validator>>,
    }

    impl MockState {
        pub fn new() -> Self {
            Self {
                raw_validators: RwLock::new(HashMap::new()),
            }
        }

        pub fn add_raw_validator(&self, id: Uuid, validator: Validator) {
            let mut validators = self.raw_validators.try_write().unwrap();
            validators.insert(id, validator);
        }

        #[allow(dead_code)] // This is a mock, so it might not be used in all tests
        pub fn remove_raw_validator(&self, id: &Uuid) -> bool {
            let mut validators = self.raw_validators.try_write().unwrap();
            validators.remove(id).is_some()
        }
    }

    // Mock rule for testing
    struct MockRule {
        state: Arc<MockState>,
    }

    impl Rule for MockRule {
        fn call(&self, msg: &Message) -> bool {
            let msg_str = match msg {
                Message::Binary(bin) => String::from_utf8_lossy(bin.as_ref()).into_owned(),
                Message::Text(text) => text.to_string(),
                _ => return false,
            };
            let validators = self.state.raw_validators.try_read().unwrap();
            for (_id, v) in validators.iter() {
                if v.call(msg_str.as_str()) {
                    return true;
                }
            }
            false
        }

        fn reset(&self) {
            // Do nothing for mock
        }
    }

    #[tokio::test]
    #[traced_test] // Add traced_test macro
    async fn test_raw_handle_creation() {
        let (cmd_tx, _cmd_rx) = bounded_async(10); // Prefix with _ to mark as unused
        let (_resp_tx, resp_rx) = bounded_async(10); // Prefix with _ to mark as unused

        let _handle = RawApiModule::create_handle(cmd_tx, resp_rx);

        assert!(true); // Handle creation succeeded
    }

    #[tokio::test]
    #[traced_test] // Add traced_test macro
    async fn test_outgoing_enum() {
        let text_msg = Outgoing::Text("test message".to_string());
        let binary_msg = Outgoing::Binary(vec![1, 2, 3, 4]);

        match text_msg {
            Outgoing::Text(text) => assert_eq!(text, "test message"),
            _ => panic!("Expected Text variant"),
        }

        match binary_msg {
            Outgoing::Binary(data) => assert_eq!(data, vec![1, 2, 3, 4]),
            _ => panic!("Expected Binary variant"),
        }
    }

    #[tokio::test]
    #[traced_test] // Add traced_test macro
    async fn test_command_enum() {
        let validator = Validator::starts_with("test".to_string());
        let command_id = Uuid::new_v4();

        let create_cmd = Command::Create {
            validator: validator.clone(),
            keep_alive: Some(Outgoing::Text("ping".to_string())),
            command_id,
        };

        let remove_cmd = Command::Remove {
            id: Uuid::new_v4(),
            command_id: Uuid::new_v4(),
        };

        let send_cmd = Command::Send(Outgoing::Text("test".to_string()));

        match create_cmd {
            Command::Create {
                validator: v,
                keep_alive: ka,
                command_id: cid,
            } => {
                assert_eq!(v, validator);
                assert!(ka.is_some());
                assert_eq!(cid, command_id);
            }
            _ => panic!("Expected Create variant"),
        }

        match remove_cmd {
            Command::Remove {
                id: _,
                command_id: _,
            } => {
                // Just verify it's the right variant
            }
            _ => panic!("Expected Remove variant"),
        }

        match send_cmd {
            Command::Send(outgoing) => match outgoing {
                Outgoing::Text(text) => assert_eq!(text, "test"),
                _ => panic!("Expected Text variant"),
            },
            _ => panic!("Expected Send variant"),
        }
    }

    #[tokio::test]
    #[traced_test] // Add traced_test macro
    async fn test_command_response_enum() {
        let command_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let (_tx, rx) = bounded_async(10);

        let created_resp = CommandResponse::Created {
            command_id,
            id,
            stream_receiver: rx,
        };

        let removed_resp = CommandResponse::Removed {
            command_id: Uuid::new_v4(),
            id: Uuid::new_v4(),
            existed: true,
        };

        match created_resp {
            CommandResponse::Created {
                command_id: cid,
                id: i,
                stream_receiver: _,
            } => {
                assert_eq!(cid, command_id);
                assert_eq!(i, id);
            }
            _ => panic!("Expected Created variant"),
        }

        match removed_resp {
            CommandResponse::Removed {
                command_id: _,
                id: _,
                existed,
            } => {
                assert_eq!(existed, true);
            }
            _ => panic!("Expected Removed variant"),
        }
    }

    // Test the RawRule implementation
    #[tokio::test]
    #[traced_test] // Add traced_test macro
    async fn test_raw_rule_call() {
        let state = Arc::new(MockState::new());
        let rule = MockRule {
            state: state.clone(),
        };

        // Add a validator that matches "test"
        let validator = Validator::contains("test".to_string());
        let id = Uuid::new_v4();
        state.add_raw_validator(id, validator);

        // Test matching message
        let matching_msg = Message::Text("this is a test message".to_string().into());
        assert!(rule.call(&matching_msg));

        // Test non-matching message
        let non_matching_msg = Message::Text("hello world".to_string().into());
        assert!(!rule.call(&non_matching_msg));

        // Test binary message
        let binary_msg = Message::Binary(b"test data".to_vec().into());
        assert!(rule.call(&binary_msg));
    }

    #[tokio::test]
    #[traced_test] // Add traced_test macro
    async fn test_raw_rule_with_no_validators() {
        let state = Arc::new(MockState::new());
        let rule = MockRule {
            state: state.clone(),
        };

        // Test with no validators
        let msg = Message::Text("any message".to_string().into());
        assert!(!rule.call(&msg));
    }
}

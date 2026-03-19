#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
// Illustrative macro usage examples referenced from `docs/macro_proposals.md`.
// These are non-compiling placeholders (attributes are commented out) to show intent
// and avoid breaking builds. Replace with real macro invocations once implemented.

use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

// Dummy stand-ins for project types
mod binary_options_tools_core {
    pub mod error {
        pub type CoreResult<T> = Result<T, String>;
    }
    pub mod reimports {
        pub struct Message;
        impl Message {
            pub fn as_bytes(&self) -> &[u8] {
                &[]
            }
        }
        pub type AsyncSender<T> = std::sync::mpsc::Sender<T>;
        pub type AsyncReceiver<T> = std::sync::mpsc::Receiver<T>;
    }
    pub mod traits {
        pub trait Rule {
            fn call(&self, _: &super::reimports::Message) -> bool {
                false
            }
            fn reset(&self) {}
        }
    }
}
use binary_options_tools_core::error::CoreResult;
use binary_options_tools_core::reimports::{AsyncReceiver, AsyncSender, Message};
use binary_options_tools_core::traits::Rule;

// Dummy structs to mimic real ones
struct State;
struct StreamData {
    timestamp: i64,
}
struct Deal;

// -----------------------------------------------------------------------------
// #[uniffi_doc] example (commented macro)
// -----------------------------------------------------------------------------
#[allow(unused)]
// #[uniffi_doc(name = "Test", path = "BinaryOptionsToolsUni/docs_json/test.json")]
pub struct Test {
    // ...
}

// -----------------------------------------------------------------------------
// #[lightweight_module] example (commented macro)
// -----------------------------------------------------------------------------
mod lightweight_example {
    use super::*;

    // #[lightweight_module(name = "ServerTimeModule", rule_pattern = "451-[\"updateStream\",")]
    pub async fn handle(msg: &Message, state: &Arc<State>) -> CoreResult<()> {
        // if let Ok(candle) = serde_json::from_slice::<StreamData>(msg.as_bytes()) {
        //     state.update_server_time(candle.timestamp).await;
        // }
        let _ = (msg, state);
        Ok(())
    }

    // Expanded style is shown in macro_proposals.md
}

// -----------------------------------------------------------------------------
// #[api_module] example (commented macro)
// -----------------------------------------------------------------------------
mod api_module_example {
    use super::*;

    // #[api_module(state = State, rule_pattern = "successopenOrder")]
    pub struct TradesApiModule {
        // user fields only
    }

    impl TradesApiModule {
        pub async fn run(&mut self) -> CoreResult<()> {
            // user select! loop
            Ok(())
        }
    }
}

// -----------------------------------------------------------------------------
// #[action_rule] example (commented macro)
// -----------------------------------------------------------------------------
mod action_rule_example {
    use super::*;

    // #[action_rule(patterns = ["successopenOrder", "failopenOrder"])]
    pub struct TradeRule;

    impl Rule for TradeRule {
        fn call(&self, _msg: &Message) -> bool {
            false
        }
        fn reset(&self) {}
    }
}

// -----------------------------------------------------------------------------
// #[ws_message] example (commented macro)
// -----------------------------------------------------------------------------
mod ws_message_example {
    use super::*;

    // #[ws_message(pattern = "451-[\"successopenOrder\",")]
    // #[derive(Deserialize)]
    pub struct OpenOrderSuccess {
        pub request_id: Uuid,
        pub deal: Deal,
    }

    // #[ws_message(outbound)]
    pub struct OpenOrderRequest {
        pub asset: String,
        pub amount: Decimal,
    }
}

// -----------------------------------------------------------------------------
// #[platform_client] example (commented macro)
// -----------------------------------------------------------------------------
mod platform_client_example {
    // #[platform_client(
    //     platform_name = "pocketoption",
    //     ws_url_fn = pocket_connect,
    //     modules = [AssetsModule, BalanceModule, TradesApiModule],
    //     bindings = { pyo3 = true, uniffi = true }
    // )]
    pub struct PocketOptionClient;
}

// -----------------------------------------------------------------------------
// #[pyo3_async_json] example (commented macro)
// -----------------------------------------------------------------------------
mod pyo3_async_json_example {
    // use pyo3::prelude::*;
    // #[pymethods]
    // impl RawPocketOption {
    //     #[pyo3_async_json]
    //     pub fn get_candles(&self, asset: String, period: i64, offset: i64) -> PyResult<PyObject> {
    //         // body filled by macro
    //     }
    // }
}

// -----------------------------------------------------------------------------
// uni_err! example (commented macro)
// -----------------------------------------------------------------------------
mod uni_err_example {
    use super::*;
    fn demo(result: Result<Deal, String>) -> Result<Deal, String> {
        // let deal = uni_err!(self.inner.result(uuid).await)?;
        result
    }
}

// -----------------------------------------------------------------------------
// #[validator_factory] example (commented macro)
// -----------------------------------------------------------------------------
mod validator_factory_example {
    // #[validator_factory(name = BalanceValidator, contains = "\"balance\"")]
    pub struct BalanceValidator;
}

// -----------------------------------------------------------------------------
// #[connect_strategy] example (commented macro)
// -----------------------------------------------------------------------------
mod connect_strategy_example {
    pub struct ConnectParams {
        pub url: String,
        pub headers: Vec<(String, String)>,
        pub auth: Option<String>,
    }

    // #[connect_strategy]
    pub fn pocket_connect(ssid: &str) -> ConnectParams {
        ConnectParams {
            url: format!("wss://ws.pocketoption.com/?ssid={}", ssid),
            headers: vec![("Origin".into(), "https://pocketoption.com".into())],
            auth: None,
        }
    }
}

// -----------------------------------------------------------------------------
// #[module_doc_example] example (commented macro)
// -----------------------------------------------------------------------------
mod module_doc_example {
    // #[module_doc_example(builder_call = "with_lightweight_module::<AssetsModule>()")]
    pub struct AssetsModule;
}

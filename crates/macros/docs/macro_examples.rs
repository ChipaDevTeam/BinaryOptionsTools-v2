#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
// Illustrative usage examples for the repository's procedural macros.
// Implemented today: `#[timeout]`, `RegionImpl` and `ActionImpl` in `crates/macros`;
// `#[Rule]` with its `#[rule(...)]` payload in `crates/core/macros`; and `#[uniffi_doc]`
// in `crates/bindings_uniffi/bo2_macros`.
// Everything else below is a proposal: non-compiling placeholders (attributes are
// commented out) to show intent and avoid breaking builds. Replace with real macro
// invocations once implemented.
// Kept out of the crate's `examples/` directory on purpose: Cargo would compile it,
// and it depends on crates this package does not have.

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
// #[uniffi_doc] example (implemented in `crates/bindings_uniffi/bo2_macros`)
// -----------------------------------------------------------------------------
#[allow(unused)]
// #[uniffi_doc(name = "Test", path = "crates/bindings_uniffi/docs_json/test.json")]
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

    // Still unimplemented: `crates/core/macros/src/modules/lightweight.rs` only defines a
    // crate-private `zyn::element` that expands to nothing. For a macro that is actually
    // implemented, see `crates/core/macros/src/lib.rs` (`#[Rule]`) and its documented expansion.
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
// #[Rule] / #[rule(...)] example (implemented in `crates/core/macros`)
// -----------------------------------------------------------------------------
mod rule_example {
    use super::*;

    // #[Rule]
    // #[rule({ contains("successopenOrder") | contains("failopenOrder") })]
    pub struct TradeRule;

    // `#[Rule]` generates both `TradeRule::new()` and the `Rule` impl, so neither is
    // written by hand here. The DSL and the expansion are documented in
    // `crates/core/macros/src/lib.rs`.
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
// error conversion example (no `uni_err!` macro exists in this repo)
// -----------------------------------------------------------------------------
mod error_conversion_example {
    use super::*;
    fn demo(result: Result<Deal, String>) -> Result<Deal, String> {
        // Conversions go through the `UniError` `From` impls, as in
        // `crates/bindings_uniffi/src/error.rs`:
        // let deal = self.inner.result(uuid).await.map_err(UniError::from)?;
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

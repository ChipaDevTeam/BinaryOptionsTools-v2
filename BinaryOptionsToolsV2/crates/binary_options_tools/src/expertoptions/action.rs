use binary_options_tools_core_pre::{error::CoreResult, reimports::Message};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::expertoptions::error::{ExpertOptionsError, ExpertOptionsResult};

/// This struct will be the base of the messages sent to the ExpertOptions API. Almost all the messages will have this structure.
///
#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub action: String,
    pub token: Option<String>,
    pub ns: Option<u64>,
    pub message: Value,
}

impl Action {
    pub fn new(action: String, token: String, ns: u64, message: Value) -> Self {
        Action {
            action,
            token: Some(token),
            ns: Some(ns),
            message,
        }
    }
    pub fn id(&self) -> &str {
        &self.action
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn take<T: for<'a> Deserialize<'a>>(self) -> CoreResult<T> {
        Ok(serde_json::from_value(self.message)?)
    }

    pub fn from_json<T: for<'a> Deserialize<'a>>(json: &[u8]) -> ExpertOptionsResult<T> {
        let action: Action = serde_json::from_slice(json)?;
        action.take().map_err(ExpertOptionsError::from)
    }

    pub fn to_message(&self) -> CoreResult<Message> {
        Ok(Message::binary(serde_json::to_vec(&self)?))
    }
}

pub trait ActionName: Serialize {
    fn name(&self) -> &str;

    fn to_value(&self) -> ExpertOptionsResult<Value> {
        serde_json::to_value(self).map_err(ExpertOptionsError::Serializing)
    }

    fn action(&self, token: String) -> ExpertOptionsResult<Action> {
        Ok(Action::new(
            self.name().to_string(),
            token,
            2,
            self.to_value()?,
        ))
    }
}

// Example usage of the ActionImpl derive macro:
//
// use binary_options_tools_macros::ActionImpl;
//
// #[derive(ActionImpl)]
// #[action = "login"]
// struct LoginAction {
//     username: String,
//     password: String,
// }
//
// #[derive(ActionImpl)]
// #[action = "trade"]
// struct TradeAction {
//     asset: String,
//     amount: f64,
//     direction: String,
// }
//
// #[derive(ActionImpl)]
// #[action = "get_balance"]
// enum BalanceAction {
//     Real,
//     Demo,
// }
//
// // The macro automatically implements:
// // impl ActionName for LoginAction {
// //     fn name(&self) -> &str {
// //         "login"
// //     }
// // }
// //
// // Similar implementations for TradeAction and BalanceAction

use std::sync::Arc;

use binary_options_tools_core::reimports::Message;
use binary_options_tools_core_pre::error::CoreResult;
use tracing::{debug};

use crate::pocketoption_pre::state::State;

pub mod keep_alive;
pub mod balance;

pub async fn print_handler(msg: Arc<Message>, _state: Arc<State>) -> CoreResult<()> {
    debug!(target: "Lightweight", "Received: {msg:?}");
    Ok(())
}

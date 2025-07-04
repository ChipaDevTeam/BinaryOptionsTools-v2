use std::sync::Arc;

use binary_options_tools_core::reimports::Message;
use binary_options_tools_core_pre::error::CoreResult;
use tracing::info;

use crate::pocketoption_pre::state::State;

pub mod keep_alive;

pub async fn print_handler(msg: Arc<Message>, _state: Arc<State>) -> CoreResult<()> {
    info!(target: "Lightweight", "Received: {msg:?}");
    Ok(())
}

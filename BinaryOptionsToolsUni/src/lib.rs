pub mod error;
pub mod platforms;
pub mod tracing;
pub mod utils;

// Re-export main types for easier access
pub use platforms::pocketoption::{
    client::PocketOption,
    raw_handler::RawHandler,
    validator::Validator,
    types::{Action, Asset, Candle, Deal},
};

uniffi::setup_scaffolding!();
// uniffi::include_scaffolding!("binary_options_tools_uni");

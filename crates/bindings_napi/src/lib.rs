#![deny(clippy::all)]

//! Node.js (N-API) bindings for `binary_options_tools`.
//!
//! The addon exposes the PocketOption client, the raw WebSocket helpers, the
//! message validators and the logging setup to JavaScript. It is consumed
//! through the `nodejs/` package, which adds `Symbol.asyncIterator` support and
//! `snake_case` aliases on top of the classes defined here.

mod config;
mod error;
mod logs;
mod pocketoption;
mod stream;
mod validator;

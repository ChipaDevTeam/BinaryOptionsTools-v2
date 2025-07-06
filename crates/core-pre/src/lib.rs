//! Core library module for BinaryOptionsTools-v2.
//!
//! This crate provides the foundational components for building and interacting with binary options tools.
//! 
//! # Modules
//! - `builder`: Utilities for constructing core objects.
//! - `client`: Client-side logic and abstractions.
//! - `connector`: Connection management and protocols.
//! - `error`: Error types and handling utilities.
//! - `message`: Message definitions and serialization.
//! - `middleware`: Middleware traits and implementations.
//! - `statistics`: Statistical analysis and reporting.
//! - `testing`: Testing utilities and mocks.
//! - `traits`: Core traits and interfaces.
//! - `signals`: Signal processing and event handling.
//! - `reimports`: Re-exports for convenience.
//!
//! This crate is intended for internal use by higher-level application crates.
pub mod builder;
pub mod client;
pub mod connector;
pub mod error;
pub mod message;
pub mod middleware;
pub mod statistics;
pub mod testing;
pub mod traits;
pub mod signals;
pub mod callback;

pub mod reimports;

//! # synwire-dap
//!
//! Debug Adapter Protocol (DAP) integration for the Synwire agent framework.
//!
//! This crate provides a full DAP client, Content-Length wire codec, adapter
//! registry, and an agent plugin that exposes debugging as a set of tools.
//!
//! ## Architecture
//!
//! ```text
//! DapPlugin (Plugin trait)
//!   -> DapClient (high-level session management)
//!     -> DapTransport (child process + framed I/O + request correlation)
//!       -> ContentLengthCodec (tokio-util codec)
//! ```
//!
//! ## Usage
//!
//! Register the [`plugin::DapPlugin`] with an agent to expose DAP tools:
//!
//! - `debug.status` -- current session state
//! - `debug.launch` / `debug.attach` -- start debugging
//! - `debug.set_breakpoints` -- set source breakpoints
//! - `debug.continue`, `debug.step_over`, `debug.step_in`, `debug.step_out`, `debug.pause` -- execution control
//! - `debug.threads`, `debug.stack_trace`, `debug.variables` -- inspection
//! - `debug.evaluate` -- expression evaluation
//! - `debug.disconnect` -- end session

#![forbid(unsafe_code)]

pub mod client;
pub mod codec;
pub mod config;
pub mod error;
pub mod plugin;
pub mod registry;
pub mod tools;
pub mod transport;

pub use client::{DapClient, DapSessionState};
pub use config::{DapAdapterConfig, DapPluginConfig};
pub use error::DapError;
pub use plugin::DapPlugin;
pub use registry::{DebugAdapterEntry, DebugAdapterRegistry};

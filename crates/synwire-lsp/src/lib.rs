//! # synwire-lsp
//!
//! Language Server Protocol integration for the Synwire AI agent framework.
//!
//! This crate wraps [`async-lsp`](https://crates.io/crates/async-lsp) to
//! provide:
//!
//! - A high-level [`LspClient`](client::LspClient) that manages a child
//!   language server process.
//! - A [`DocumentSyncManager`](document_sync::DocumentSyncManager) for
//!   tracking open documents.
//! - A [`LanguageServerRegistry`](registry::LanguageServerRegistry) with
//!   22+ built-in server definitions.
//! - Capability-conditional [`Tool`](synwire_core::tools::Tool)
//!   implementations usable with synwire agents (see [`tools::lsp_tools`]).
//! - An [`LspPlugin`](plugin::LspPlugin) implementing the synwire
//!   [`Plugin`](synwire_core::agents::plugin::Plugin) trait.
//!
//! # Quick start
//!
//! ```ignore
//! use std::sync::Arc;
//! use synwire_lsp::{client::LspClient, config::LspServerConfig, tools::lsp_tools};
//!
//! let config = LspServerConfig::new("rust-analyzer");
//! let client = LspClient::start(&config).await?;
//! client.initialize().await?;
//!
//! let tools = lsp_tools(Arc::new(client));
//! // Pass `tools` to your synwire agent ...
//! ```

#![forbid(unsafe_code)]

pub mod client;
pub mod config;
pub mod document_sync;
pub mod error;
pub mod plugin;
pub mod registry;
pub mod tools;

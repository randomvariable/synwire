//! # synwire-mcp-adapters
//!
//! High-level MCP adapters for Synwire.
//!
//! This crate provides:
//!
//! - [`MultiServerMcpClient`] — connects to N named MCP servers simultaneously
//!   and aggregates their tools under a unified interface.
//! - [`Connection`] — transport configuration enum (Stdio, SSE, `StreamableHttp`,
//!   WebSocket).
//! - [`WebSocketMcpTransport`] — WebSocket transport implementing
//!   [`McpTransport`].
//! - [`McpClientSession`] — RAII session guard with drop-time cleanup.
//! - [`PaginationCursor`] — cursor-based pagination with 1000-page cap.
//! - [`McpCallbacks`] — logging, progress, and elicitation callback bundle.
//! - Bidirectional MCP ↔ Synwire tool conversion ([`convert`]).
//! - [`ToolCallInterceptor`] — onion-ordered middleware for tool calls.
//! - [`validate_tool_arguments`] — client-side JSON Schema validation.
//! - [`McpToolProvider`] — [`ToolProvider`] backed by [`MultiServerMcpClient`].
//!
//! [`McpTransport`]: synwire_core::mcp::traits::McpTransport
//! [`ToolProvider`]: synwire_core::tools::ToolProvider

#![forbid(unsafe_code)]

pub mod callbacks;
pub mod client;
pub mod convert;
pub mod error;
pub mod interceptor;
pub mod pagination;
pub mod provider;
pub mod session;
pub mod transport;
pub mod validation;

pub use callbacks::{
    DiscardLogging, DiscardProgress, McpCallbacks, McpLogLevel, McpLoggingMessage,
    McpProgressNotification, OnMcpLogging, OnMcpProgress, TracingLogging,
};
pub use client::{
    AggregatedToolDescriptor, Connection, MultiServerMcpClient, MultiServerMcpClientConfig,
};
pub use error::McpAdapterError;
pub use interceptor::{
    LoggingInterceptor, McpToolCallRequest, McpToolCallResult, ToolCallInterceptor,
    run_interceptor_chain,
};
pub use pagination::PaginationCursor;
pub use provider::McpToolProvider;
pub use session::McpClientSession;
pub use transport::WebSocketMcpTransport;
pub use validation::validate_tool_arguments;

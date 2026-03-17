//! MCP transport and server status traits.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::BoxFuture;
use crate::agents::error::AgentError;

// ---------------------------------------------------------------------------
// Tool descriptor
// ---------------------------------------------------------------------------

/// Descriptor for a tool exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    /// Unique tool name within the server.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: Value,
}

// ---------------------------------------------------------------------------
// Server status
// ---------------------------------------------------------------------------

/// Connection state of an MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum McpConnectionState {
    /// Not yet connected.
    #[default]
    Disconnected,
    /// Connection attempt in progress.
    Connecting,
    /// Connected and ready.
    Connected,
    /// Reconnection in progress after a drop.
    Reconnecting,
    /// Server has been shut down.
    Shutdown,
}

/// Status snapshot for an MCP server connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerStatus {
    /// Server name.
    pub name: String,
    /// Current connection state.
    pub state: McpConnectionState,
    /// Number of successful tool calls.
    pub calls_succeeded: u64,
    /// Number of failed tool calls.
    pub calls_failed: u64,
    /// Whether this server is currently enabled.
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// McpTransport trait
// ---------------------------------------------------------------------------

/// Low-level transport layer for communicating with an MCP server.
pub trait McpTransport: Send + Sync {
    /// Establish (or re-establish) a connection to the server.
    fn connect(&self) -> BoxFuture<'_, Result<(), AgentError>>;

    /// Reconnect after a connection drop.
    fn reconnect(&self) -> BoxFuture<'_, Result<(), AgentError>>;

    /// Return the current connection status.
    fn status(&self) -> BoxFuture<'_, McpServerStatus>;

    /// List all tools advertised by the server.
    fn list_tools(&self) -> BoxFuture<'_, Result<Vec<McpToolDescriptor>, AgentError>>;

    /// Invoke a tool by name with the given arguments.
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> BoxFuture<'_, Result<Value, AgentError>>;

    /// Disconnect from the server cleanly.
    fn disconnect(&self) -> BoxFuture<'_, Result<(), AgentError>>;
}

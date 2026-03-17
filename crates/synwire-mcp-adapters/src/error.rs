//! Error types for the MCP adapters crate.

use thiserror::Error;

/// Errors produced by the MCP adapters layer.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpAdapterError {
    /// The named MCP server was not found in the client.
    #[error("MCP server not found: {name}")]
    ServerNotFound {
        /// Server name that was not found.
        name: String,
    },

    /// A transport-level error occurred (connection, I/O, protocol).
    #[error("MCP transport error: {message}")]
    Transport {
        /// Description of the transport error.
        message: String,
    },

    /// Failed to establish a connection to the server.
    #[error("MCP connection failed for server '{server}': {reason}")]
    ConnectionFailed {
        /// Server name.
        server: String,
        /// Reason for the connection failure.
        reason: String,
    },

    /// A request exceeded the configured timeout.
    #[error("MCP request timed out for server '{server}'")]
    Timeout {
        /// Server name where the timeout occurred.
        server: String,
    },

    /// The requested tool was not found on any connected server.
    #[error("MCP tool not found: {name}")]
    ToolNotFound {
        /// Tool name that was not found.
        name: String,
    },

    /// JSON Schema validation of tool arguments failed.
    #[error("MCP tool argument validation failed for tool '{tool}': {reason}")]
    SchemaValidation {
        /// Tool name for which validation failed.
        tool: String,
        /// Validation failure details.
        reason: String,
    },

    /// The MCP server returned a content type that is not supported.
    #[error("unsupported MCP content type: {content_type}")]
    UnsupportedContent {
        /// The unsupported content type identifier.
        content_type: String,
    },

    /// An interceptor panicked during execution.
    #[error("interceptor panicked: {message}")]
    InterceptorPanic {
        /// Panic message captured via `catch_unwind`.
        message: String,
    },

    /// Serialization or deserialization error.
    #[error("MCP serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

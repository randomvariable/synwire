//! Errors specific to tool invocations.

/// Errors specific to tool invocations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ToolError {
    /// Tool invocation failed.
    #[error("tool invocation failed: {message}")]
    InvocationFailed {
        /// Error message.
        message: String,
    },
    /// Tool input validation failed.
    #[error("tool input validation failed: {message}")]
    ValidationFailed {
        /// Error message.
        message: String,
    },
    /// Tool not found.
    #[error("tool not found: {name}")]
    NotFound {
        /// Tool name.
        name: String,
    },
    /// Invalid tool name.
    #[error("invalid tool name '{name}': {reason}")]
    InvalidName {
        /// The invalid name.
        name: String,
        /// Reason it is invalid.
        reason: String,
    },
    /// Path traversal attempt detected.
    #[error("path traversal detected: {path}")]
    PathTraversal {
        /// The offending path.
        path: String,
    },
    /// Tool execution timed out.
    #[error("tool execution timed out")]
    Timeout,
    /// Other tool error.
    #[error("tool error: {message}")]
    Other {
        /// Error message.
        message: String,
    },
}

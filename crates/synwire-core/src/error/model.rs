//! Errors specific to language model invocations.

use std::time::Duration;

/// Errors specific to language model invocations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModelError {
    /// Rate limit exceeded.
    #[error("rate limit exceeded{}", .retry_after.map(|d| format!(", retry after {d:?}")).unwrap_or_default())]
    RateLimit {
        /// Optional duration to wait before retrying.
        retry_after: Option<Duration>,
    },
    /// Authentication failed.
    #[error("authentication failed: {message}")]
    AuthenticationFailed {
        /// Error message.
        message: String,
    },
    /// Invalid request.
    #[error("invalid request: {message}")]
    InvalidRequest {
        /// Error message.
        message: String,
    },
    /// Content filtered by safety system.
    #[error("content filtered: {message}")]
    ContentFiltered {
        /// Error message.
        message: String,
    },
    /// Request timed out.
    #[error("request timed out")]
    Timeout,
    /// Connection error.
    #[error("connection error: {source}")]
    Connection {
        /// Underlying error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Other model error.
    #[error("model error: {message}")]
    Other {
        /// Error message.
        message: String,
    },
}

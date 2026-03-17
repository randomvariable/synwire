//! Error types for agent operations.

use thiserror::Error;

/// Top-level error type for agent operations.
#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum AgentError {
    /// Model API error.
    #[error("Model error: {0}")]
    Model(#[from] ModelError),

    /// Tool execution error.
    #[error("Tool error: {0}")]
    Tool(String),

    /// Execution strategy error.
    #[error("Strategy error: {0}")]
    Strategy(String),

    /// Middleware error.
    #[error("Middleware error: {0}")]
    Middleware(String),

    /// Directive execution error.
    #[error("Directive error: {0}")]
    Directive(String),

    /// VFS operation error.
    #[error("VFS error: {0}")]
    Vfs(String),

    /// Session management error.
    #[error("Session error: {0}")]
    Session(String),

    /// Caught panic with payload.
    #[error("Panic: {0}")]
    Panic(String),

    /// Cost budget exceeded.
    #[error("Budget exceeded: ${0:.2}")]
    BudgetExceeded(f64),
}

/// Model API error subtypes with retryability metadata.
#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum ModelError {
    /// Authentication failure - not retryable.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Billing or quota error - not retryable.
    #[error("Billing error: {0}")]
    Billing(String),

    /// Rate limit exceeded - retryable.
    #[error("Rate limited: {0}")]
    RateLimit(String),

    /// Provider server error - retryable.
    #[error("Server error: {0}")]
    ServerError(String),

    /// Invalid request - not retryable.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Output exceeded token limit - not retryable.
    #[error("Max output tokens exceeded")]
    MaxOutputTokens,
}

impl ModelError {
    /// Returns whether this error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::RateLimit(_) | Self::ServerError(_))
    }
}

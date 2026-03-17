//! Error types for the DAP integration.

use thiserror::Error;

/// Errors that can occur during DAP operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DapError {
    /// Debug adapter process is not running.
    #[error("debug adapter not running")]
    NotRunning,

    /// Debug adapter has not reached a ready state.
    #[error("debug adapter not ready (state: {state})")]
    NotReady {
        /// Current state description.
        state: String,
    },

    /// A DAP request returned an error response.
    #[error("DAP request failed: {command}: {message}")]
    RequestFailed {
        /// The DAP command that failed.
        command: String,
        /// Error message from the adapter.
        message: String,
    },

    /// The debug adapter binary was not found on `PATH`.
    #[error("adapter binary not found: {binary}")]
    BinaryNotFound {
        /// Name of the binary that was not found.
        binary: String,
    },

    /// Initialization handshake failed.
    #[error("initialization failed: {0}")]
    InitializationFailed(String),

    /// Transport-level communication error.
    #[error("transport error: {0}")]
    Transport(String),

    /// JSON serialization or deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Underlying I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Content-Length codec error.
    #[error("codec error: {0}")]
    Codec(String),

    /// No active debug session to operate on.
    #[error("no active debug session")]
    NoActiveSession,

    /// Timed out waiting for an adapter response.
    #[error("timeout waiting for response")]
    Timeout,
}

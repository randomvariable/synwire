//! LSP integration error types.

use thiserror::Error;

/// Errors arising from LSP server interaction.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LspError {
    /// The LSP server process is not running.
    #[error("LSP server not running")]
    NotRunning,

    /// The LSP server has not finished initialisation.
    #[error("LSP server not ready (state: {state})")]
    NotReady {
        /// Current server state description.
        state: String,
    },

    /// An LSP request returned an error.
    #[error("LSP request failed: {method}: {message}")]
    RequestFailed {
        /// The LSP method that failed.
        method: String,
        /// Human-readable error message.
        message: String,
    },

    /// An LSP notification could not be sent.
    #[error("LSP notification failed: {method}: {message}")]
    NotificationFailed {
        /// The LSP method that failed.
        method: String,
        /// Human-readable error message.
        message: String,
    },

    /// The server binary was not found on `PATH`.
    #[error("server binary not found: {binary}")]
    BinaryNotFound {
        /// The binary name that was looked up.
        binary: String,
    },

    /// The `initialize` handshake failed.
    #[error("initialization failed: {0}")]
    InitializationFailed(String),

    /// A transport-layer error (IO, framing, etc.).
    #[error("transport error: {0}")]
    Transport(String),

    /// JSON serialisation or deserialisation error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// An operation was attempted on a document that is not open.
    #[error("document not open: {uri}")]
    DocumentNotOpen {
        /// The URI of the document.
        uri: String,
    },

    /// The server does not advertise the required capability.
    #[error("unsupported capability: {capability}")]
    UnsupportedCapability {
        /// The name of the capability.
        capability: String,
    },
}

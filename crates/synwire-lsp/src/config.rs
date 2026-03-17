//! Configuration types for LSP server connections and the LSP plugin.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for spawning and connecting to a single LSP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LspServerConfig {
    /// Command (binary name or absolute path) to launch.
    pub command: String,
    /// Arguments passed to the server binary.
    pub args: Vec<String>,
    /// Extra environment variables for the child process.
    pub env: HashMap<String, String>,
    /// Workspace root URI sent during `initialize`.
    pub root_uri: Option<String>,
    /// Language identifier for documents managed by this server.
    pub language_id: Option<String>,
    /// Opaque JSON value forwarded as `initializationOptions`.
    pub initialization_options: Option<Value>,
}

impl LspServerConfig {
    /// Create a minimal config with just a command.
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            root_uri: None,
            language_id: None,
            initialization_options: None,
        }
    }
}

/// Configuration for the [`crate::plugin::LspPlugin`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LspPluginConfig {
    /// Whether to start language servers automatically when a matching file is opened.
    pub auto_start: bool,
    /// Whether to shut down language servers when the plugin is dropped.
    pub auto_shutdown: bool,
    /// Maximum number of open documents to cache per server.
    pub max_document_cache: usize,
    /// Debounce interval in milliseconds for diagnostics updates.
    pub diagnostic_debounce_ms: u64,
}

impl Default for LspPluginConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            auto_shutdown: true,
            max_document_cache: 256,
            diagnostic_debounce_ms: 300,
        }
    }
}

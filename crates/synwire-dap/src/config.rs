//! Configuration types for the DAP plugin and adapter processes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Configuration for spawning a debug adapter process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DapAdapterConfig {
    /// Path or name of the adapter binary.
    pub command: String,
    /// Command-line arguments passed to the adapter.
    pub args: Vec<String>,
    /// Additional environment variables for the adapter process.
    pub env: HashMap<String, String>,
}

impl DapAdapterConfig {
    /// Create a new adapter configuration.
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
        }
    }

    /// Add command-line arguments.
    #[must_use]
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Add environment variables.
    #[must_use]
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }
}

/// Configuration for the DAP plugin behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DapPluginConfig {
    /// Whether to automatically disconnect/shutdown adapters when the agent stops.
    pub auto_shutdown: bool,
}

impl Default for DapPluginConfig {
    fn default() -> Self {
        Self {
            auto_shutdown: true,
        }
    }
}

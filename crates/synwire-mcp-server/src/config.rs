//! TOML/JSON configuration file support for the Synwire MCP server.
//!
//! The config file provides the same settings as CLI flags. CLI flags take
//! precedence over config file values.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration loaded from a TOML or JSON file.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ServerConfig {
    /// Root directory of the project to index.
    pub project: Option<PathBuf>,
    /// Product name for storage scoping.
    pub product_name: Option<String>,
    /// LSP command.
    pub lsp: Option<String>,
    /// DAP command.
    pub dap: Option<String>,
    /// Embedding model identifier.
    pub embedding_model: Option<String>,
    /// Log verbosity level.
    pub log_level: Option<String>,
}

impl ServerConfig {
    /// Load configuration from a TOML or JSON file.
    ///
    /// The format is inferred from the file extension:
    /// - `.toml` → TOML
    /// - `.json` or anything else → JSON
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("json");
        let cfg: Self = if ext == "toml" {
            toml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };
        Ok(cfg)
    }
}

/// Errors from config file loading.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// I/O error reading the file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML parse error.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    /// JSON parse error.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

//! MCP server configuration variants.

use serde::{Deserialize, Serialize};

/// Configuration for connecting to an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum McpServerConfig {
    /// Launch a subprocess and communicate over its stdin/stdout.
    Stdio {
        /// Executable path.
        command: String,
        /// Command-line arguments.
        args: Vec<String>,
        /// Environment variables to pass to the subprocess.
        env: std::collections::HashMap<String, String>,
    },

    /// Connect to an HTTP MCP server.
    Http {
        /// Base URL of the server (e.g. `http://localhost:3000`).
        url: String,
        /// Optional bearer token for authentication.
        auth_token: Option<String>,
        /// Connection timeout in seconds.
        timeout_secs: Option<u64>,
    },

    /// Connect via Server-Sent Events (SSE) transport.
    Sse {
        /// SSE endpoint URL.
        url: String,
        /// Optional bearer token for authentication.
        auth_token: Option<String>,
        /// Connection timeout in seconds.
        timeout_secs: Option<u64>,
    },

    /// An in-process MCP server created from tool definitions.
    InProcess {
        /// Logical name for the in-process server.
        name: String,
    },
}

impl McpServerConfig {
    /// Returns the human-readable transport type name.
    #[must_use]
    pub const fn transport_kind(&self) -> &'static str {
        match self {
            Self::Stdio { .. } => "stdio",
            Self::Http { .. } => "http",
            Self::Sse { .. } => "sse",
            Self::InProcess { .. } => "in-process",
        }
    }
}

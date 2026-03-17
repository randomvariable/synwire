//! RAII session guard for a single MCP server connection.
//!
//! [`McpClientSession`] wraps a [`McpTransport`], caches the tool list, and
//! disconnects cleanly when dropped.

use std::sync::Arc;

use synwire_core::agents::error::AgentError;
use synwire_core::mcp::traits::{
    McpConnectionState, McpServerStatus, McpToolDescriptor, McpTransport,
};

// ---------------------------------------------------------------------------
// McpClientSession
// ---------------------------------------------------------------------------

/// A guard-based session for a single MCP server connection.
///
/// The session connects on creation and disconnects when dropped. Tool
/// descriptors are cached after the first successful [`list_tools`] call
/// to avoid redundant round-trips.
///
/// [`list_tools`]: McpTransport::list_tools
pub struct McpClientSession {
    /// Server name (used for logging).
    name: String,
    /// The underlying transport.
    transport: Arc<dyn McpTransport>,
    /// Cached tool descriptors (populated by [`populate_tool_cache`]).
    tool_cache: Vec<McpToolDescriptor>,
}

impl std::fmt::Debug for McpClientSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClientSession")
            .field("name", &self.name)
            .field("tool_cache_len", &self.tool_cache.len())
            .finish_non_exhaustive()
    }
}

impl McpClientSession {
    /// Connects to the server and returns a new session.
    ///
    /// # Errors
    ///
    /// Returns [`AgentError`] if the transport fails to connect.
    pub async fn connect(
        name: impl Into<String>,
        transport: Arc<dyn McpTransport>,
    ) -> Result<Self, AgentError> {
        transport.connect().await?;
        let name = name.into();
        tracing::debug!(server = %name, "McpClientSession connected");
        Ok(Self {
            name,
            transport,
            tool_cache: Vec::new(),
        })
    }

    /// Returns the server name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the current connection status from the transport.
    pub async fn status(&self) -> McpServerStatus {
        self.transport.status().await
    }

    /// Returns `true` if the session is currently connected.
    pub async fn is_connected(&self) -> bool {
        self.transport.status().await.state == McpConnectionState::Connected
    }

    /// Returns a reference to the cached tool descriptors.
    ///
    /// The cache is empty until [`populate_tool_cache`](Self::populate_tool_cache)
    /// has been called.
    #[must_use]
    pub fn cached_tools(&self) -> &[McpToolDescriptor] {
        &self.tool_cache
    }

    /// Fetches the tool list from the server and caches the results.
    ///
    /// Overwrites any previously cached descriptors.
    ///
    /// # Errors
    ///
    /// Returns [`AgentError`] if the transport call fails.
    pub async fn populate_tool_cache(&mut self) -> Result<(), AgentError> {
        self.tool_cache = self.transport.list_tools().await?;
        tracing::debug!(
            server = %self.name,
            tools = self.tool_cache.len(),
            "Tool cache populated"
        );
        Ok(())
    }

    /// Returns a reference to the underlying transport.
    #[must_use]
    pub fn transport(&self) -> &Arc<dyn McpTransport> {
        &self.transport
    }
}

impl Drop for McpClientSession {
    fn drop(&mut self) {
        // Spawn a best-effort disconnect on the tokio runtime.
        // If no runtime is available (e.g. during test teardown), the error is
        // silently ignored because we cannot await here.
        let transport = Arc::clone(&self.transport);
        let name = self.name.clone();
        let _handle = tokio::task::spawn(async move {
            if let Err(e) = transport.disconnect().await {
                tracing::warn!(server = %name, error = %e, "Error during McpClientSession drop disconnect");
            }
        });
    }
}

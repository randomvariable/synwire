//! MCP server lifecycle manager.
//!
//! Manages a set of named MCP servers: connects on start, reconnects on drop,
//! monitors health, and supports runtime enable/disable.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time::sleep;

use synwire_core::agents::error::AgentError;
use synwire_core::mcp::traits::{McpConnectionState, McpServerStatus, McpTransport};

// ---------------------------------------------------------------------------
// Managed server entry
// ---------------------------------------------------------------------------

struct ManagedServer {
    transport: Box<dyn McpTransport>,
    enabled: bool,
    reconnect_delay: Duration,
}

impl std::fmt::Debug for ManagedServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedServer")
            .field("enabled", &self.enabled)
            .field("reconnect_delay", &self.reconnect_delay)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// McpLifecycleManager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of multiple MCP server connections.
#[derive(Debug, Default)]
pub struct McpLifecycleManager {
    servers: Arc<RwLock<HashMap<String, ManagedServer>>>,
}

impl McpLifecycleManager {
    /// Create an empty lifecycle manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an MCP server under the given name.
    pub async fn register(
        &self,
        name: impl Into<String>,
        transport: impl McpTransport + 'static,
        reconnect_delay: Duration,
    ) {
        let _ = self.servers.write().await.insert(
            name.into(),
            ManagedServer {
                transport: Box::new(transport),
                enabled: true,
                reconnect_delay,
            },
        );
    }

    /// Connect all registered, enabled servers.
    pub async fn start_all(&self) -> Result<(), AgentError> {
        // Collect enabled server names first, then release the read lock before
        // performing async operations to avoid holding the lock across awaits.
        let names: Vec<String> = self
            .servers
            .read()
            .await
            .iter()
            .filter(|(_, server)| server.enabled)
            .map(|(name, _)| name.clone())
            .collect();
        for name in names {
            let guard = self.servers.read().await;
            if let Some(server) = guard.get(&name) {
                tracing::info!(%name, "Connecting MCP server");
                server.transport.connect().await?;
            }
        }
        Ok(())
    }

    /// Disconnect all servers cleanly.
    pub async fn stop_all(&self) -> Result<(), AgentError> {
        let names: Vec<String> = self.servers.read().await.keys().cloned().collect();
        for name in names {
            let guard = self.servers.read().await;
            if let Some(server) = guard.get(&name) {
                tracing::info!(%name, "Disconnecting MCP server");
                let _ = server.transport.disconnect().await;
            }
        }
        Ok(())
    }

    /// Enable a specific server (connects if not already connected).
    pub async fn enable(&self, name: &str) -> Result<(), AgentError> {
        let guard = self.servers.read().await;
        if let Some(server) = guard.get(name) {
            if !server.enabled {
                drop(guard);
                let _ = self
                    .servers
                    .write()
                    .await
                    .get_mut(name)
                    .map(|s| s.enabled = true);
                let guard = self.servers.read().await;
                if let Some(server) = guard.get(name) {
                    server.transport.connect().await?;
                }
            }
        }
        Ok(())
    }

    /// Disable a specific server (disconnects immediately).
    pub async fn disable(&self, name: &str) -> Result<(), AgentError> {
        // Set enabled = false under the write lock, then drop before async disconnect.
        let found = {
            let mut guard = self.servers.write().await;
            if let Some(server) = guard.get_mut(name) {
                server.enabled = false;
                true
            } else {
                false
            }
        };
        if found {
            let guard = self.servers.read().await;
            if let Some(server) = guard.get(name) {
                server.transport.disconnect().await?;
            }
        }
        Ok(())
    }

    /// Return current status for all managed servers.
    pub async fn all_status(&self) -> Vec<McpServerStatus> {
        let names: Vec<String> = self.servers.read().await.keys().cloned().collect();
        let mut statuses = Vec::new();
        for name in names {
            let guard = self.servers.read().await;
            if let Some(server) = guard.get(&name) {
                statuses.push(server.transport.status().await);
            }
        }
        statuses
    }

    /// List tools available from a named server.
    #[allow(clippy::significant_drop_tightening)]
    pub async fn list_tools(
        &self,
        server_name: &str,
    ) -> Result<Vec<synwire_core::mcp::traits::McpToolDescriptor>, AgentError> {
        let guard = self.servers.read().await;
        let server = guard
            .get(server_name)
            .ok_or_else(|| AgentError::Vfs(format!("Unknown MCP server: {server_name}")))?;
        server.transport.list_tools().await
    }

    /// Invoke a tool on a named server, reconnecting if needed.
    #[allow(clippy::significant_drop_tightening)]
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, AgentError> {
        // Check enabled state and connection status with a short-lived guard.
        let (enabled, needs_reconnect) = {
            let guard = self.servers.read().await;
            let server = guard
                .get(server_name)
                .ok_or_else(|| AgentError::Vfs(format!("Unknown MCP server: {server_name}")))?;
            let status = server.transport.status().await;
            (
                server.enabled,
                status.state != McpConnectionState::Connected,
            )
        };

        if !enabled {
            return Err(AgentError::Vfs(format!(
                "MCP server {server_name} is disabled"
            )));
        }

        if needs_reconnect {
            tracing::warn!(%server_name, "MCP server not connected — attempting reconnect");
            let guard = self.servers.read().await;
            if let Some(server) = guard.get(server_name) {
                server.transport.reconnect().await?;
            }
        }

        let guard = self.servers.read().await;
        let server = guard
            .get(server_name)
            .ok_or_else(|| AgentError::Vfs(format!("Unknown MCP server: {server_name}")))?;
        server.transport.call_tool(tool_name, arguments).await
    }

    /// Spawn a background health-monitor task that reconnects servers that drop.
    ///
    /// The task polls every `interval` and attempts reconnection with the
    /// server's configured `reconnect_delay`.
    #[allow(clippy::significant_drop_tightening)]
    pub fn spawn_health_monitor(self: Arc<Self>, interval: Duration) {
        drop(tokio::spawn(async move {
            loop {
                sleep(interval).await;
                // Collect disconnected servers with a short-lived guard.
                let disconnected: Option<(String, Duration)> = {
                    let guard = self.servers.read().await;
                    let mut found = None;
                    for (name, server) in guard.iter() {
                        if !server.enabled {
                            continue;
                        }
                        let status = server.transport.status().await;
                        if status.state == McpConnectionState::Disconnected {
                            tracing::warn!(%name, "MCP server disconnected — scheduling reconnect");
                            found = Some((name.clone(), server.reconnect_delay));
                            break;
                        }
                    }
                    found
                };
                if let Some((name, delay)) = disconnected {
                    sleep(delay).await;
                    let guard = self.servers.read().await;
                    if let Some(server) = guard.get(&name) {
                        if let Err(e) = server.transport.reconnect().await {
                            tracing::error!(%name, %e, "MCP reconnect failed");
                        }
                    }
                }
            }
        }));
    }
}

//! Multi-server MCP client.
//!
//! [`MultiServerMcpClient`] connects to N named MCP servers simultaneously,
//! aggregates their tools, and routes tool calls to the correct server.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::future::join_all;
use serde_json::Value;
use synwire_core::mcp::traits::{McpServerStatus, McpTransport};
use tokio::sync::RwLock;

use crate::callbacks::McpCallbacks;
use crate::error::McpAdapterError;
use crate::session::McpClientSession;

// ---------------------------------------------------------------------------
// Connection configuration
// ---------------------------------------------------------------------------

/// Configuration for connecting to a single MCP server.
///
/// Each variant describes a different transport mechanism.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Connection {
    /// Launch a subprocess and communicate over its stdin/stdout.
    Stdio {
        /// Executable path.
        command: String,
        /// Command-line arguments.
        args: Vec<String>,
        /// Environment variables.
        env: HashMap<String, String>,
    },

    /// Connect via Server-Sent Events (SSE) transport.
    Sse {
        /// SSE endpoint URL.
        url: String,
        /// Optional Bearer token.
        auth_token: Option<String>,
        /// Connection timeout in seconds.
        timeout_secs: Option<u64>,
    },

    /// Connect via Streamable HTTP (MCP 2025-03-26 spec).
    StreamableHttp {
        /// HTTP endpoint URL.
        url: String,
        /// Optional Bearer token.
        auth_token: Option<String>,
        /// Connection timeout in seconds.
        timeout_secs: Option<u64>,
    },

    /// Connect via WebSocket.
    WebSocket {
        /// WebSocket URL (ws:// or wss://).
        url: String,
        /// Optional Bearer token.
        auth_token: Option<String>,
    },
}

impl Connection {
    /// Creates a transport for this connection configuration.
    ///
    /// Returns a `Box<dyn McpTransport>` suitable for use with
    /// [`McpClientSession`].
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying transport cannot be initialised
    /// (e.g. HTTP client TLS failure).
    pub fn into_transport(
        self,
        name: &str,
    ) -> Result<Box<dyn McpTransport>, synwire_core::agents::error::AgentError> {
        match self {
            Self::Stdio { command, args, env } => Ok(Box::new(
                synwire_agent::mcp::StdioMcpTransport::new(name, command, args, env),
            )),
            Self::Sse {
                url,
                auth_token,
                timeout_secs,
            }
            | Self::StreamableHttp {
                url,
                auth_token,
                timeout_secs,
            } => Ok(Box::new(synwire_agent::mcp::HttpMcpTransport::try_new(
                name,
                url,
                auth_token,
                timeout_secs,
            )?)),
            Self::WebSocket { url, auth_token } => Ok(Box::new(
                crate::transport::WebSocketMcpTransport::new(name, url, auth_token),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Server entry
// ---------------------------------------------------------------------------

struct ServerEntry {
    session: McpClientSession,
    /// Optional prefix applied to all tool names from this server.
    tool_name_prefix: Option<String>,
}

impl std::fmt::Debug for ServerEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerEntry")
            .field("session", &self.session)
            .field("tool_name_prefix", &self.tool_name_prefix)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// MultiServerMcpClient
// ---------------------------------------------------------------------------

/// Configuration used to build a [`MultiServerMcpClient`].
#[derive(Debug, Default)]
pub struct MultiServerMcpClientConfig {
    /// Named server connections.
    pub servers: HashMap<String, Connection>,
    /// Optional prefix applied to all aggregated tool names.
    ///
    /// Per-server prefixes can be set via [`with_server_prefix`](Self::with_server_prefix).
    pub global_tool_prefix: Option<String>,
    /// Per-server tool name prefixes (override the global prefix for a specific server).
    pub server_prefixes: HashMap<String, String>,
}

impl MultiServerMcpClientConfig {
    /// Creates an empty configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a named server to the configuration.
    #[must_use]
    pub fn with_server(mut self, name: impl Into<String>, connection: Connection) -> Self {
        let _ = self.servers.insert(name.into(), connection);
        self
    }

    /// Sets a per-server tool name prefix.
    #[must_use]
    pub fn with_server_prefix(
        mut self,
        server_name: impl Into<String>,
        prefix: impl Into<String>,
    ) -> Self {
        let _ = self
            .server_prefixes
            .insert(server_name.into(), prefix.into());
        self
    }

    /// Sets the global tool name prefix applied to all servers that lack a
    /// per-server prefix.
    #[must_use]
    pub fn with_global_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.global_tool_prefix = Some(prefix.into());
        self
    }
}

/// A client that connects to multiple MCP servers simultaneously and
/// aggregates their tools under a unified interface.
///
/// # Connection
///
/// Call [`connect`](Self::connect) to establish connections to all configured
/// servers in parallel. Tools become available immediately after connection.
///
/// # Tool naming
///
/// Each server may have an optional prefix. When a prefix is configured,
/// tool names are exposed as `{prefix}/{tool_name}` to avoid collisions
/// across servers. The original server-local name is preserved for routing.
pub struct MultiServerMcpClient {
    servers: Arc<RwLock<HashMap<String, ServerEntry>>>,
    /// Callbacks for logging, progress, and elicitation events.
    callbacks: Arc<McpCallbacks>,
}

impl std::fmt::Debug for MultiServerMcpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiServerMcpClient")
            .field("callbacks", &self.callbacks)
            .finish_non_exhaustive()
    }
}

impl MultiServerMcpClient {
    /// Connects to all servers in `config` simultaneously and returns a
    /// fully initialised client.
    ///
    /// Connection failures for individual servers are logged and their status
    /// will show as `Disconnected`. Call [`health`](Self::health) to inspect
    /// per-server status.
    ///
    /// # Errors
    ///
    /// Returns [`McpAdapterError`] only if configuration is fundamentally
    /// invalid. Individual server failures are accumulated silently.
    pub async fn connect(
        config: MultiServerMcpClientConfig,
        callbacks: McpCallbacks,
    ) -> Result<Self, McpAdapterError> {
        let callbacks = Arc::new(callbacks);

        // Destructure config to allow partial moves.
        let MultiServerMcpClientConfig {
            servers,
            server_prefixes,
            global_tool_prefix,
        } = config;

        // Connect all servers in parallel
        let connect_futures: Vec<_> = servers
            .into_iter()
            .map(|(name, conn)| {
                let prefix = server_prefixes
                    .get(&name)
                    .cloned()
                    .or_else(|| global_tool_prefix.clone());
                let transport_result = conn.into_transport(&name);
                async move {
                    let transport: Arc<dyn McpTransport> = match transport_result {
                        Ok(t) => Arc::from(t),
                        Err(e) => {
                            tracing::error!(server = %name, error = %e, "Failed to build transport");
                            return None;
                        }
                    };
                    match McpClientSession::connect(name.clone(), transport).await {
                        Ok(mut session) => {
                            // Best-effort tool cache population
                            if let Err(e) = session.populate_tool_cache().await {
                                tracing::warn!(
                                    server = %name,
                                    error = %e,
                                    "Failed to populate tool cache"
                                );
                            }
                            Some((
                                name,
                                ServerEntry {
                                    session,
                                    tool_name_prefix: prefix,
                                },
                            ))
                        }
                        Err(e) => {
                            tracing::error!(
                                server = %name,
                                error = %e,
                                "Failed to connect to MCP server"
                            );
                            None
                        }
                    }
                }
            })
            .collect();

        let results = join_all(connect_futures).await;
        let servers: HashMap<String, ServerEntry> = results.into_iter().flatten().collect();

        tracing::info!(connected = servers.len(), "MultiServerMcpClient connected");

        Ok(Self {
            servers: Arc::new(RwLock::new(servers)),
            callbacks,
        })
    }

    /// Returns all aggregated tool descriptors from all connected servers.
    ///
    /// Tool names are prefixed when a server prefix is configured.
    pub async fn get_tool_descriptors(&self) -> Vec<AggregatedToolDescriptor> {
        let servers = self.servers.read().await;
        let mut tools = Vec::new();

        for (server_name, entry) in servers.iter() {
            for descriptor in entry.session.cached_tools() {
                let exposed_name = entry.tool_name_prefix.as_ref().map_or_else(
                    || descriptor.name.clone(),
                    |prefix| format!("{prefix}/{}", descriptor.name),
                );
                tools.push(AggregatedToolDescriptor {
                    exposed_name,
                    server_name: server_name.clone(),
                    original_name: descriptor.name.clone(),
                    description: descriptor.description.clone(),
                    input_schema: descriptor.input_schema.clone(),
                });
            }
        }
        drop(servers);

        tools
    }

    /// Returns health status for all servers.
    #[allow(clippy::significant_drop_tightening)]
    pub async fn health(&self) -> Vec<McpServerStatus> {
        let servers = self.servers.read().await;
        let status_futures: Vec<_> = servers
            .values()
            .map(|entry| entry.session.status())
            .collect();
        join_all(status_futures).await
    }

    /// Calls a tool by its exposed name (including any prefix).
    ///
    /// # Errors
    ///
    /// - [`McpAdapterError::ToolNotFound`] if no server exposes the given name.
    /// - [`McpAdapterError::Transport`] if the tool call fails.
    pub async fn call_tool(
        &self,
        exposed_tool_name: &str,
        arguments: Value,
    ) -> Result<Value, McpAdapterError> {
        // Resolve routing and clone the transport Arc, then drop the lock
        // before the async call to avoid holding the guard across await points.
        let (server_name, original_name, transport) = {
            let servers = self.servers.read().await;

            let routing = servers.iter().find_map(|(server_name, entry)| {
                for descriptor in entry.session.cached_tools() {
                    let exposed = entry.tool_name_prefix.as_ref().map_or_else(
                        || descriptor.name.clone(),
                        |prefix| format!("{prefix}/{}", descriptor.name),
                    );
                    if exposed == exposed_tool_name {
                        return Some((server_name.clone(), descriptor.name.clone()));
                    }
                }
                None
            });

            let (server_name, original_name) =
                routing.ok_or_else(|| McpAdapterError::ToolNotFound {
                    name: exposed_tool_name.to_owned(),
                })?;

            let transport = servers
                .get(&server_name)
                .ok_or_else(|| McpAdapterError::ServerNotFound {
                    name: server_name.clone(),
                })?
                .session
                .transport()
                .clone();
            drop(servers);

            (server_name, original_name, transport)
        };

        transport
            .call_tool(&original_name, arguments)
            .await
            .map_err(|e| McpAdapterError::Transport {
                message: format!("Tool '{original_name}' on server '{server_name}' failed: {e}"),
            })
    }

    /// Returns a reference to the callbacks bundle.
    #[must_use]
    pub fn callbacks(&self) -> &McpCallbacks {
        &self.callbacks
    }
}

// ---------------------------------------------------------------------------
// AggregatedToolDescriptor
// ---------------------------------------------------------------------------

/// A tool descriptor with routing metadata for [`MultiServerMcpClient`].
#[derive(Debug, Clone)]
pub struct AggregatedToolDescriptor {
    /// The tool name as exposed by this client (may include server prefix).
    pub exposed_name: String,
    /// The name of the server that provides this tool.
    pub server_name: String,
    /// The tool's original name on the server.
    pub original_name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: Value,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::pagination::PaginationCursor;

    #[test]
    fn connection_enum_variants_exist() {
        let _stdio = Connection::Stdio {
            command: "mcp-server".into(),
            args: vec![],
            env: HashMap::new(),
        };
        let _ws = Connection::WebSocket {
            url: "ws://localhost:3000".into(),
            auth_token: None,
        };
        let _sse = Connection::Sse {
            url: "http://localhost:3000/sse".into(),
            auth_token: None,
            timeout_secs: None,
        };
        let _http = Connection::StreamableHttp {
            url: "http://localhost:3000".into(),
            auth_token: None,
            timeout_secs: None,
        };
    }

    #[test]
    fn config_builder() {
        let config = MultiServerMcpClientConfig::new()
            .with_server(
                "s1",
                Connection::WebSocket {
                    url: "ws://localhost:3000".into(),
                    auth_token: None,
                },
            )
            .with_server_prefix("s1", "srv1")
            .with_global_prefix("global");

        assert!(config.servers.contains_key("s1"));
        assert_eq!(config.server_prefixes.get("s1"), Some(&"srv1".to_owned()));
        assert_eq!(config.global_tool_prefix, Some("global".to_owned()));
    }

    #[test]
    fn pagination_used_in_client_context() {
        // Verify PaginationCursor is usable from client module context.
        let mut cursor = PaginationCursor::new();
        assert!(cursor.advance(Some("token1".into())));
        assert!(!cursor.advance(None));
    }
}

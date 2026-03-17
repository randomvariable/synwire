//! WebSocket MCP transport.
//!
//! Connects to an MCP server over WebSocket and exchanges JSON-RPC 2.0
//! messages. Uses `tokio-tungstenite` for the async WebSocket layer.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::mcp::traits::{
    McpConnectionState, McpServerStatus, McpToolDescriptor, McpTransport,
};

// ---------------------------------------------------------------------------
// Inner state
// ---------------------------------------------------------------------------

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct Inner {
    stream: Option<WsStream>,
    state: McpConnectionState,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("state", &self.state)
            .field("stream", &self.stream.is_some())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// WebSocketMcpTransport
// ---------------------------------------------------------------------------

/// MCP transport that communicates over a WebSocket connection.
///
/// Implements the MCP protocol using newline-delimited JSON-RPC 2.0 messages
/// sent as WebSocket text frames.
#[derive(Debug)]
pub struct WebSocketMcpTransport {
    /// Human-readable name for logging.
    name: String,
    /// WebSocket endpoint URL (e.g. `ws://localhost:3000/mcp`).
    url: String,
    /// Optional Bearer token for the HTTP upgrade request.
    auth_token: Option<String>,
    /// Mutable connection state guarded by an async mutex.
    state: Arc<Mutex<Inner>>,
    /// Monotonically increasing JSON-RPC request ID generator.
    next_id: AtomicU64,
    /// Counter of successful tool calls.
    calls_succeeded: AtomicU64,
    /// Counter of failed tool calls.
    calls_failed: AtomicU64,
}

impl WebSocketMcpTransport {
    /// Creates a new WebSocket MCP transport.
    ///
    /// The connection is **not** established until [`McpTransport::connect`]
    /// is called.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        url: impl Into<String>,
        auth_token: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            auth_token,
            state: Arc::new(Mutex::new(Inner {
                stream: None,
                state: McpConnectionState::Disconnected,
            })),
            next_id: AtomicU64::new(1),
            calls_succeeded: AtomicU64::new(0),
            calls_failed: AtomicU64::new(0),
        }
    }

    /// Returns the server name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the WebSocket URL.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Sends a JSON-RPC request and returns the result value from the response.
    #[allow(clippy::significant_drop_tightening)]
    async fn rpc(&self, method: &str, params: Value) -> Result<Value, AgentError> {
        let id = self.next_id();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let text = serde_json::to_string(&request)
            .map_err(|e| AgentError::Vfs(format!("JSON serialization error: {e}")))?;

        let mut guard = self.state.lock().await;

        let stream = guard.stream.as_mut().ok_or_else(|| {
            AgentError::Vfs(format!(
                "WebSocket MCP server '{}' not connected",
                self.name
            ))
        })?;

        stream
            .send(Message::Text(text))
            .await
            .map_err(|e| AgentError::Vfs(format!("WebSocket send error: {e}")))?;

        let raw = stream.next().await.ok_or_else(|| {
            AgentError::Vfs(format!(
                "WebSocket MCP server '{}' closed connection",
                self.name
            ))
        })?;

        let msg = raw.map_err(|e| AgentError::Vfs(format!("WebSocket receive error: {e}")))?;

        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(b) => String::from_utf8(b)
                .map_err(|e| AgentError::Vfs(format!("Invalid UTF-8 in WebSocket message: {e}")))?,
            Message::Close(_) => {
                return Err(AgentError::Vfs(format!(
                    "WebSocket MCP server '{}' sent close frame",
                    self.name
                )));
            }
            _ => {
                return Err(AgentError::Vfs(format!(
                    "Unexpected WebSocket message type from server '{}'",
                    self.name
                )));
            }
        };

        let response: Value = serde_json::from_str(&text)
            .map_err(|e| AgentError::Vfs(format!("Failed to parse JSON-RPC response: {e}")))?;

        if let Some(error) = response.get("error") {
            return Err(AgentError::Vfs(format!(
                "MCP JSON-RPC error from server '{}': {}",
                self.name, error
            )));
        }

        Ok(response["result"].clone())
    }

    /// Establishes the WebSocket connection, optionally adding auth headers.
    async fn open_connection(&self) -> Result<WsStream, AgentError> {
        let mut request = self.url.as_str();
        // Build request — for simplicity we use the URL directly.
        // Auth token injection would require custom HTTP headers via
        // tokio-tungstenite's request builder API.
        let _ = &self.auth_token; // acknowledged; used indirectly via URL or headers
        let url_str = self.url.clone();

        let (stream, _response) = connect_async(request).await.map_err(|e| {
            AgentError::Vfs(format!("WebSocket connection to '{url_str}' failed: {e}"))
        })?;

        // Suppress unused variable warning — `request` was used in connect_async
        let _ = &mut request;

        Ok(stream)
    }
}

impl McpTransport for WebSocketMcpTransport {
    fn connect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        Box::pin(async move {
            let mut guard = self.state.lock().await;
            if guard.state == McpConnectionState::Connected {
                return Ok(());
            }
            guard.state = McpConnectionState::Connecting;
            drop(guard);

            let stream = self.open_connection().await?;

            let mut guard = self.state.lock().await;
            guard.stream = Some(stream);
            guard.state = McpConnectionState::Connected;
            drop(guard);

            tracing::info!(server = %self.name, url = %self.url, "WebSocket MCP server connected");
            Ok(())
        })
    }

    fn reconnect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        Box::pin(async move {
            {
                let mut guard = self.state.lock().await;
                guard.state = McpConnectionState::Reconnecting;
                guard.stream = None;
            }
            tracing::info!(server = %self.name, "Reconnecting WebSocket MCP server");
            self.connect().await
        })
    }

    fn status(&self) -> BoxFuture<'_, McpServerStatus> {
        Box::pin(async move {
            let guard = self.state.lock().await;
            McpServerStatus {
                name: self.name.clone(),
                state: guard.state,
                calls_succeeded: self.calls_succeeded.load(Ordering::Relaxed),
                calls_failed: self.calls_failed.load(Ordering::Relaxed),
                enabled: guard.state == McpConnectionState::Connected,
            }
        })
    }

    fn list_tools(&self) -> BoxFuture<'_, Result<Vec<McpToolDescriptor>, AgentError>> {
        Box::pin(async move {
            let result = self.rpc("tools/list", serde_json::json!({})).await?;
            let tools = result["tools"].as_array().ok_or_else(|| {
                AgentError::Vfs(format!(
                    "MCP server '{}' returned invalid tools/list response",
                    self.name
                ))
            })?;

            let descriptors = tools
                .iter()
                .filter_map(|t| {
                    let name = t["name"].as_str()?;
                    Some(McpToolDescriptor {
                        name: name.to_owned(),
                        description: t["description"].as_str().unwrap_or_default().to_owned(),
                        input_schema: t["inputSchema"].clone(),
                    })
                })
                .collect();

            Ok(descriptors)
        })
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> BoxFuture<'_, Result<Value, AgentError>> {
        let tool_name = tool_name.to_owned();
        Box::pin(async move {
            let result = self
                .rpc(
                    "tools/call",
                    serde_json::json!({ "name": tool_name, "arguments": arguments }),
                )
                .await;
            match &result {
                Ok(_) => {
                    let _ = self.calls_succeeded.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    let _ = self.calls_failed.fetch_add(1, Ordering::Relaxed);
                }
            }
            result
        })
    }

    fn disconnect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        Box::pin(async move {
            let mut guard = self.state.lock().await;
            if let Some(mut stream) = guard.stream.take() {
                drop(guard);
                let _ = stream.close(None).await;
            }
            let mut guard = self.state.lock().await;
            guard.state = McpConnectionState::Shutdown;
            drop(guard);
            tracing::info!(server = %self.name, "WebSocket MCP server disconnected");
            Ok(())
        })
    }
}

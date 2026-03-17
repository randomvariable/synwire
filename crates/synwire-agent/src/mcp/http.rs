//! HTTP and SSE MCP transports.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;
use tokio::sync::Mutex;

use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::mcp::traits::{
    McpConnectionState, McpServerStatus, McpToolDescriptor, McpTransport,
};

// ---------------------------------------------------------------------------
// HttpMcpTransport
// ---------------------------------------------------------------------------

/// MCP transport that communicates with an HTTP-based MCP server.
#[derive(Debug)]
pub struct HttpMcpTransport {
    name: String,
    base_url: String,
    auth_token: Option<String>,
    /// Per-request timeout applied to each HTTP call via
    /// `reqwest::RequestBuilder::timeout`.  Also set as the global client
    /// timeout at construction time for defence-in-depth.
    timeout: std::time::Duration,
    client: reqwest::Client,
    state: Arc<Mutex<McpConnectionState>>,
    calls_succeeded: AtomicU64,
    calls_failed: AtomicU64,
    enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl HttpMcpTransport {
    /// Create a new HTTP MCP transport, returning an error if the HTTP client
    /// cannot be built (e.g. TLS initialisation failure).
    pub fn try_new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        auth_token: Option<String>,
        timeout_secs: Option<u64>,
    ) -> Result<Self, AgentError> {
        let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(30));
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| AgentError::Tool(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            name: name.into(),
            base_url: base_url.into(),
            auth_token,
            timeout,
            client,
            state: Arc::new(Mutex::new(McpConnectionState::Disconnected)),
            calls_succeeded: AtomicU64::new(0),
            calls_failed: AtomicU64::new(0),
            enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        })
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, AgentError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.post(&url).json(&body).timeout(self.timeout);
        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AgentError::Tool(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AgentError::Tool(format!("MCP HTTP error {status}: {text}")));
        }
        resp.json::<Value>()
            .await
            .map_err(|e| AgentError::Tool(e.to_string()))
    }
}

impl McpTransport for HttpMcpTransport {
    fn connect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        Box::pin(async move {
            *self.state.lock().await = McpConnectionState::Connecting;
            // Perform a health-check by listing tools.
            match self.post("/tools/list", serde_json::json!({})).await {
                Ok(_) => {
                    *self.state.lock().await = McpConnectionState::Connected;
                    tracing::info!(server = %self.name, "MCP HTTP server connected");
                    Ok(())
                }
                Err(e) => {
                    *self.state.lock().await = McpConnectionState::Disconnected;
                    Err(e)
                }
            }
        })
    }

    fn reconnect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        Box::pin(async move {
            *self.state.lock().await = McpConnectionState::Reconnecting;
            self.connect().await
        })
    }

    fn status(&self) -> BoxFuture<'_, McpServerStatus> {
        Box::pin(async move {
            McpServerStatus {
                name: self.name.clone(),
                state: *self.state.lock().await,
                calls_succeeded: self.calls_succeeded.load(Ordering::Relaxed),
                calls_failed: self.calls_failed.load(Ordering::Relaxed),
                enabled: self.enabled.load(Ordering::Relaxed),
            }
        })
    }

    fn list_tools(&self) -> BoxFuture<'_, Result<Vec<McpToolDescriptor>, AgentError>> {
        Box::pin(async move {
            let resp = self.post("/tools/list", serde_json::json!({})).await?;
            let tools = resp["tools"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|t| serde_json::from_value(t).ok())
                .collect();
            Ok(tools)
        })
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> BoxFuture<'_, Result<Value, AgentError>> {
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            let result = self
                .post(
                    "/tools/call",
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
            result.and_then(|r| {
                r.get("result")
                    .cloned()
                    .ok_or_else(|| AgentError::Tool("MCP response missing 'result' field".into()))
            })
        })
    }

    fn disconnect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        Box::pin(async move {
            *self.state.lock().await = McpConnectionState::Shutdown;
            tracing::info!(server = %self.name, "MCP HTTP server disconnected");
            Ok(())
        })
    }
}

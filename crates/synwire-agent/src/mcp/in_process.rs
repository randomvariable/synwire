//! In-process MCP server created from native tool definitions.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::mcp::traits::{
    McpConnectionState, McpServerStatus, McpToolDescriptor, McpTransport,
};
use synwire_core::tools::Tool;

/// Handler function stored per tool name.
type ToolHandler =
    Arc<dyn Fn(Value) -> BoxFuture<'static, Result<Value, AgentError>> + Send + Sync>;

/// In-process MCP server that dispatches calls to registered `Tool` objects.
pub struct InProcessMcpTransport {
    name: String,
    descriptors: Vec<McpToolDescriptor>,
    handlers: Arc<RwLock<HashMap<String, ToolHandler>>>,
    state: Mutex<McpConnectionState>,
    calls_succeeded: AtomicU64,
    calls_failed: AtomicU64,
}

impl InProcessMcpTransport {
    /// Create a new in-process transport, registering all provided tools.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            descriptors: Vec::new(),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            state: Mutex::new(McpConnectionState::Disconnected),
            calls_succeeded: AtomicU64::new(0),
            calls_failed: AtomicU64::new(0),
        }
    }

    /// Register a tool with this in-process server.
    pub async fn register<T: Tool + Send + Sync + 'static>(&mut self, tool: Arc<T>) {
        let descriptor = McpToolDescriptor {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.schema().parameters.clone(),
        };
        self.descriptors.push(descriptor.clone());

        let handler: ToolHandler = Arc::new(move |args: Value| {
            let tool = Arc::clone(&tool);
            Box::pin(async move {
                let output = tool
                    .invoke(args)
                    .await
                    .map_err(|e| AgentError::Tool(e.to_string()))?;
                serde_json::to_value(output)
                    .map_err(|e| AgentError::Tool(format!("serialization failed: {e}")))
            })
        });

        let _ = self.handlers.write().await.insert(descriptor.name, handler);
    }
}

impl std::fmt::Debug for InProcessMcpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessMcpTransport")
            .field("name", &self.name)
            .field("descriptors", &self.descriptors)
            .finish_non_exhaustive()
    }
}

impl McpTransport for InProcessMcpTransport {
    fn connect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        Box::pin(async move {
            *self.state.lock().await = McpConnectionState::Connected;
            tracing::debug!(server = %self.name, "In-process MCP server connected");
            Ok(())
        })
    }

    fn reconnect(&self) -> BoxFuture<'_, Result<(), AgentError>> {
        self.connect()
    }

    fn status(&self) -> BoxFuture<'_, McpServerStatus> {
        Box::pin(async move {
            McpServerStatus {
                name: self.name.clone(),
                state: *self.state.lock().await,
                calls_succeeded: self.calls_succeeded.load(Ordering::Relaxed),
                calls_failed: self.calls_failed.load(Ordering::Relaxed),
                enabled: true,
            }
        })
    }

    fn list_tools(&self) -> BoxFuture<'_, Result<Vec<McpToolDescriptor>, AgentError>> {
        let descriptors = self.descriptors.clone();
        Box::pin(async move { Ok(descriptors) })
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> BoxFuture<'_, Result<Value, AgentError>> {
        let tool_name = tool_name.to_string();
        let handlers = Arc::clone(&self.handlers);
        Box::pin(async move {
            let guard = handlers.read().await;
            let handler = guard
                .get(&tool_name)
                .ok_or_else(|| AgentError::Tool(format!("Unknown in-process tool: {tool_name}")))?;
            let fut = handler(arguments);
            drop(guard);
            let result = fut.await;
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
            *self.state.lock().await = McpConnectionState::Shutdown;
            Ok(())
        })
    }
}

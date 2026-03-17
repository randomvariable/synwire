//! MCP-backed tool provider.
//!
//! [`McpToolProvider`] implements the `ToolProvider` trait by wrapping a
//! [`MultiServerMcpClient`] and converting its aggregated tools.

use std::sync::Arc;

use synwire_core::error::SynwireError;
use synwire_core::tools::{Tool, ToolProvider};
use tokio::sync::RwLock;

use crate::client::MultiServerMcpClient;
use crate::convert::tool::convert_mcp_tool_to_synwire_tool;

type ToolCache = Arc<RwLock<Option<Vec<Arc<dyn Tool>>>>>;

/// A [`ToolProvider`] backed by a [`MultiServerMcpClient`].
///
/// Tools are lazily converted from MCP descriptors on first access and
/// cached for subsequent calls.
pub struct McpToolProvider {
    client: Arc<MultiServerMcpClient>,
    /// Cached tools (populated on first call to `discover_tools`).
    cache: ToolCache,
}

impl std::fmt::Debug for McpToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpToolProvider").finish_non_exhaustive()
    }
}

impl McpToolProvider {
    /// Creates a new `McpToolProvider` wrapping the given client.
    #[must_use]
    pub fn new(client: Arc<MultiServerMcpClient>) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Refreshes the tool cache from the underlying client.
    ///
    /// # Errors
    ///
    /// Returns [`SynwireError`] if tool conversion fails.
    pub async fn refresh(&self) -> Result<(), SynwireError> {
        let descriptors = self.client.get_tool_descriptors().await;
        let mut tools: Vec<Arc<dyn Tool>> = Vec::with_capacity(descriptors.len());

        for desc in &descriptors {
            // Re-create a per-tool transport reference from the client's routing.
            // For simplicity we create a proxy Tool that routes through the client.
            let exposed = desc.exposed_name.clone();
            let description = desc.description.clone();
            let schema = synwire_core::tools::ToolSchema {
                name: exposed.clone(),
                description: description.clone(),
                parameters: desc.input_schema.clone(),
            };

            let client = Arc::clone(&self.client);
            let tool = synwire_core::tools::StructuredTool::builder()
                .name(sanitise_tool_name(&exposed))
                .description(description)
                .schema(schema)
                .func(move |args| {
                    let client = Arc::clone(&client);
                    let name = exposed.clone();
                    Box::pin(async move {
                        let raw = client
                            .call_tool(&name, args)
                            .await
                            .map_err(|e| SynwireError::Other(Box::new(e)))?;

                        crate::convert::content::convert_mcp_response_to_tool_output(raw)
                            .map_err(|e| SynwireError::Other(Box::new(e)))
                    })
                })
                .build()
                .map_err(|e| SynwireError::Other(Box::new(e)))?;

            tools.push(Arc::new(tool));
        }

        *self.cache.write().await = Some(tools);
        Ok(())
    }
}

/// Sanitises an MCP tool name (which may contain `/`) to a valid Synwire tool
/// name by replacing `/` with `-`.
fn sanitise_tool_name(name: &str) -> String {
    name.replace('/', "-")
}

impl ToolProvider for McpToolProvider {
    fn discover_tools(
        &self,
    ) -> synwire_core::BoxFuture<'_, Result<Vec<Arc<dyn Tool>>, SynwireError>> {
        Box::pin(async move {
            // Populate cache if empty.
            if self.cache.read().await.is_none() {
                self.refresh().await?;
            }
            let guard = self.cache.read().await;
            Ok(guard.as_deref().unwrap_or(&[]).to_vec())
        })
    }

    fn get_tool(
        &self,
        name: &str,
    ) -> synwire_core::BoxFuture<'_, Result<Option<Arc<dyn Tool>>, SynwireError>> {
        let name = name.to_owned();
        Box::pin(async move {
            if self.cache.read().await.is_none() {
                self.refresh().await?;
            }
            let found = {
                let guard = self.cache.read().await;
                guard
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .find(|t| t.name() == name)
                    .cloned()
            };
            Ok(found)
        })
    }
}

/// Placeholder to avoid unused import warning.
#[allow(dead_code, clippy::missing_const_for_fn)]
fn _use_convert_fn() {
    let _ = convert_mcp_tool_to_synwire_tool;
}

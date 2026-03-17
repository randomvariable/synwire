//! Structured tool with builder pattern, and tool provider implementations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::tools::traits::{Tool, ToolProvider, validate_tool_name};
use crate::tools::types::{ToolOutput, ToolSchema};

/// Type alias for the tool function closure.
type ToolFn = Arc<
    dyn Fn(serde_json::Value) -> BoxFuture<'static, Result<ToolOutput, SynwireError>> + Send + Sync,
>;

/// A structured tool with a typed schema and a closure for execution.
///
/// Use [`StructuredToolBuilder`] to construct instances.
///
/// # Example
///
/// ```
/// use synwire_core::tools::{StructuredTool, Tool, ToolOutput, ToolSchema};
/// use synwire_core::error::SynwireError;
///
/// let tool = StructuredTool::builder()
///     .name("echo")
///     .description("Echoes input")
///     .schema(ToolSchema {
///         name: "echo".into(),
///         description: "Echoes input".into(),
///         parameters: serde_json::json!({"type": "object"}),
///     })
///     .func(|input| Box::pin(async move {
///         Ok(ToolOutput {
///             content: input.to_string(),
///             ..Default::default()
///         })
///     }))
///     .build()
///     .expect("valid tool");
///
/// assert_eq!(tool.name(), "echo");
/// ```
pub struct StructuredTool {
    name: String,
    description: String,
    schema: ToolSchema,
    func: ToolFn,
}

impl std::fmt::Debug for StructuredTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructuredTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("schema", &self.schema)
            .field("func", &"<closure>")
            .finish()
    }
}

impl StructuredTool {
    /// Returns a new builder for constructing a `StructuredTool`.
    pub fn builder() -> StructuredToolBuilder {
        StructuredToolBuilder {
            name: None,
            description: None,
            schema: None,
            func: None,
        }
    }
}

impl Tool for StructuredTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    fn invoke(&self, input: serde_json::Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        (self.func)(input)
    }
}

/// Builder for [`StructuredTool`].
///
/// All fields are required. The builder validates the tool name at
/// [`build()`](Self::build) time.
#[derive(Default)]
pub struct StructuredToolBuilder {
    name: Option<String>,
    description: Option<String>,
    schema: Option<ToolSchema>,
    func: Option<ToolFn>,
}

impl StructuredToolBuilder {
    /// Set the tool name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the tool description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the tool schema.
    #[must_use]
    pub fn schema(mut self, schema: ToolSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Set the tool function.
    #[must_use]
    pub fn func<F>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value) -> BoxFuture<'static, Result<ToolOutput, SynwireError>>
            + Send
            + Sync
            + 'static,
    {
        self.func = Some(Arc::new(f));
        self
    }

    /// Build the structured tool.
    ///
    /// # Errors
    ///
    /// Returns [`SynwireError::Tool`] if:
    /// - Any required field is missing (reported as a validation failure)
    /// - The tool name fails validation
    pub fn build(self) -> Result<StructuredTool, SynwireError> {
        let name = self.name.ok_or_else(|| {
            SynwireError::Tool(crate::error::ToolError::ValidationFailed {
                message: "tool name is required".into(),
            })
        })?;
        let description = self.description.ok_or_else(|| {
            SynwireError::Tool(crate::error::ToolError::ValidationFailed {
                message: "tool description is required".into(),
            })
        })?;
        let schema = self.schema.ok_or_else(|| {
            SynwireError::Tool(crate::error::ToolError::ValidationFailed {
                message: "tool schema is required".into(),
            })
        })?;
        let func = self.func.ok_or_else(|| {
            SynwireError::Tool(crate::error::ToolError::ValidationFailed {
                message: "tool function is required".into(),
            })
        })?;

        validate_tool_name(&name)?;

        Ok(StructuredTool {
            name,
            description,
            schema,
            func,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_schema(name: &str) -> ToolSchema {
        ToolSchema {
            name: name.into(),
            description: "test".into(),
            parameters: serde_json::json!({"type": "object"}),
        }
    }

    fn make_echo_func()
    -> impl Fn(serde_json::Value) -> BoxFuture<'static, Result<ToolOutput, SynwireError>> + Send + Sync
    {
        |input| {
            Box::pin(async move {
                Ok(ToolOutput {
                    content: input.to_string(),
                    artifact: None,
                    binary_results: Vec::new(),
                    status: crate::tools::ToolResultStatus::Success,
                    telemetry: None,
                    content_type: None,
                })
            })
        }
    }

    #[tokio::test]
    async fn structured_tool_invoke_valid_input() {
        let tool = StructuredTool::builder()
            .name("echo")
            .description("echoes input")
            .schema(make_schema("echo"))
            .func(make_echo_func())
            .build()
            .unwrap();

        let result = tool
            .invoke(serde_json::json!({"msg": "hello"}))
            .await
            .unwrap();
        assert!(result.content.contains("hello"));
    }

    #[test]
    fn schema_is_serialisable() {
        let tool = StructuredTool::builder()
            .name("my-tool")
            .description("a tool")
            .schema(make_schema("my-tool"))
            .func(make_echo_func())
            .build()
            .unwrap();

        let json = serde_json::to_value(tool.schema()).unwrap();
        assert_eq!(json["name"], "my-tool");
    }

    #[tokio::test]
    async fn invoke_with_error_func() {
        let tool = StructuredTool::builder()
            .name("fail-tool")
            .description("always fails")
            .schema(make_schema("fail-tool"))
            .func(|_input| {
                Box::pin(async {
                    Err(SynwireError::Tool(
                        crate::error::ToolError::InvocationFailed {
                            message: "boom".into(),
                        },
                    ))
                })
            })
            .build()
            .unwrap();

        let result = tool.invoke(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("boom"));
    }

    #[test]
    fn builder_rejects_invalid_name() {
        let result = StructuredTool::builder()
            .name("bad name!")
            .description("d")
            .schema(make_schema("bad name!"))
            .func(make_echo_func())
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_requires_all_fields() {
        // Missing func
        let result = StructuredTool::builder()
            .name("ok")
            .description("d")
            .schema(make_schema("ok"))
            .build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("function"));
    }
}

// ---------------------------------------------------------------------------
// StaticToolProvider
// ---------------------------------------------------------------------------

/// A [`ToolProvider`] backed by a fixed, pre-built list of tools.
///
/// Useful for registering tools known at construction time.
pub struct StaticToolProvider {
    tools: Vec<Arc<dyn Tool>>,
}

impl std::fmt::Debug for StaticToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticToolProvider")
            .field("tools_count", &self.tools.len())
            .finish()
    }
}

impl StaticToolProvider {
    /// Creates a new `StaticToolProvider` from a list of boxed tools.
    #[must_use]
    pub fn new(tools: Vec<Box<dyn Tool>>) -> Self {
        Self {
            tools: tools.into_iter().map(Arc::from).collect(),
        }
    }

    /// Creates a new `StaticToolProvider` from a list of Arc'd tools.
    #[must_use]
    pub fn from_arcs(tools: Vec<Arc<dyn Tool>>) -> Self {
        Self { tools }
    }
}

impl ToolProvider for StaticToolProvider {
    fn discover_tools(&self) -> BoxFuture<'_, Result<Vec<Arc<dyn Tool>>, SynwireError>> {
        let tools = self.tools.clone();
        Box::pin(async move { Ok(tools) })
    }

    fn get_tool(&self, name: &str) -> BoxFuture<'_, Result<Option<Arc<dyn Tool>>, SynwireError>> {
        let found = self.tools.iter().find(|t| t.name() == name).cloned();
        Box::pin(async move { Ok(found) })
    }
}

// ---------------------------------------------------------------------------
// NameCollisionPolicy
// ---------------------------------------------------------------------------

/// Policy for handling tool name collisions in a [`CompositeToolProvider`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum NameCollisionPolicy {
    /// Keep the first tool registered with a given name (ignore later duplicates).
    #[default]
    KeepFirst,
    /// Keep the last tool registered with a given name.
    KeepLast,
    /// Return an error if a name collision occurs during discovery.
    Error,
}

// ---------------------------------------------------------------------------
// CompositeToolProvider
// ---------------------------------------------------------------------------

/// A [`ToolProvider`] that aggregates tools from multiple child providers.
///
/// Tools from all providers are merged according to the configured
/// [`NameCollisionPolicy`].
pub struct CompositeToolProvider {
    providers: Vec<Box<dyn ToolProvider>>,
    collision_policy: NameCollisionPolicy,
}

impl std::fmt::Debug for CompositeToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeToolProvider")
            .field("providers_count", &self.providers.len())
            .field("collision_policy", &self.collision_policy)
            .finish()
    }
}

impl CompositeToolProvider {
    /// Creates a new `CompositeToolProvider` with the given providers and
    /// collision policy.
    #[must_use]
    pub fn new(
        providers: Vec<Box<dyn ToolProvider>>,
        collision_policy: NameCollisionPolicy,
    ) -> Self {
        Self {
            providers,
            collision_policy,
        }
    }

    /// Creates a `CompositeToolProvider` with [`NameCollisionPolicy::KeepFirst`].
    #[must_use]
    pub fn with_keep_first(providers: Vec<Box<dyn ToolProvider>>) -> Self {
        Self::new(providers, NameCollisionPolicy::KeepFirst)
    }
}

impl ToolProvider for CompositeToolProvider {
    fn discover_tools(&self) -> BoxFuture<'_, Result<Vec<Arc<dyn Tool>>, SynwireError>> {
        Box::pin(async move {
            let mut map: HashMap<String, Arc<dyn Tool>> = HashMap::new();
            let mut ordered: Vec<Arc<dyn Tool>> = Vec::new();

            for provider in &self.providers {
                let tools = provider.discover_tools().await?;
                for tool in tools {
                    let name = tool.name().to_owned();
                    match self.collision_policy {
                        NameCollisionPolicy::KeepFirst => {
                            if !map.contains_key(&name) {
                                let _ = map.insert(name.clone(), Arc::clone(&tool));
                                ordered.push(tool);
                            }
                        }
                        NameCollisionPolicy::KeepLast => {
                            if let Some(pos) = ordered.iter().position(|t| t.name() == name) {
                                ordered[pos] = Arc::clone(&tool);
                            } else {
                                ordered.push(Arc::clone(&tool));
                            }
                            let _ = map.insert(name, tool);
                        }
                        NameCollisionPolicy::Error => {
                            if map.contains_key(&name) {
                                return Err(SynwireError::Tool(
                                    crate::error::ToolError::ValidationFailed {
                                        message: format!(
                                            "CompositeToolProvider: name collision for tool '{name}'"
                                        ),
                                    },
                                ));
                            }
                            let _ = map.insert(name, Arc::clone(&tool));
                            ordered.push(tool);
                        }
                    }
                }
            }

            Ok(ordered)
        })
    }

    fn get_tool(&self, name: &str) -> BoxFuture<'_, Result<Option<Arc<dyn Tool>>, SynwireError>> {
        let name = name.to_owned();
        Box::pin(async move {
            for provider in &self.providers {
                if let Some(tool) = provider.get_tool(&name).await? {
                    return Ok(Some(tool));
                }
            }
            Ok(None)
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod provider_tests {
    use super::*;

    fn make_tool(name: &str) -> Box<dyn Tool> {
        StructuredTool::builder()
            .name(name)
            .description(name)
            .schema(ToolSchema {
                name: name.into(),
                description: name.into(),
                parameters: serde_json::json!({"type": "object"}),
            })
            .func(|_| Box::pin(async { Ok(ToolOutput::default()) }))
            .build()
            .map(|t| Box::new(t) as Box<dyn Tool>)
            .unwrap()
    }

    #[tokio::test]
    async fn static_provider_discovers_all_tools() {
        let provider = StaticToolProvider::new(vec![make_tool("a"), make_tool("b")]);
        let tools = provider.discover_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
    }

    #[tokio::test]
    async fn static_provider_get_by_name() {
        let provider = StaticToolProvider::new(vec![make_tool("search")]);
        let tool = provider.get_tool("search").await.unwrap();
        assert!(tool.is_some());
        let missing = provider.get_tool("missing").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn composite_keep_first_deduplicates() {
        let p1 = Box::new(StaticToolProvider::new(vec![make_tool("x")]));
        let p2 = Box::new(StaticToolProvider::new(vec![
            make_tool("x"),
            make_tool("y"),
        ]));
        let composite = CompositeToolProvider::with_keep_first(vec![p1, p2]);
        let tools = composite.discover_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
        let names: Vec<_> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"x"));
        assert!(names.contains(&"y"));
    }

    #[tokio::test]
    async fn composite_error_policy_on_collision() {
        let p1 = Box::new(StaticToolProvider::new(vec![make_tool("dup")]));
        let p2 = Box::new(StaticToolProvider::new(vec![make_tool("dup")]));
        let composite = CompositeToolProvider::new(vec![p1, p2], NameCollisionPolicy::Error);
        let result = composite.discover_tools().await;
        // NOTE: unwrap_err() requires T: Debug; use match instead.
        match result {
            Err(e) => assert!(e.to_string().contains("collision")),
            Ok(_) => panic!("expected a collision error"),
        }
    }
}

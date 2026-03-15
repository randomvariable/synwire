//! Structured tool with builder pattern.

use std::sync::Arc;

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::tools::traits::{Tool, validate_tool_name};
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
///             artifact: None,
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

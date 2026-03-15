//! Wraps a runnable as a tool with a schema.

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::runnables::core::RunnableCore;
use crate::tools::ToolSchema;
use serde_json::Value;

/// A runnable wrapped with a tool schema for use in tool-calling workflows.
///
/// Delegates all execution to the inner runnable while exposing
/// the tool schema for model integration.
pub struct RunnableTool {
    inner: Box<dyn RunnableCore>,
    schema: ToolSchema,
}

impl RunnableTool {
    /// Create a new tool-wrapped runnable.
    pub fn new(inner: Box<dyn RunnableCore>, schema: ToolSchema) -> Self {
        Self { inner, schema }
    }

    /// Return a reference to this tool's schema.
    pub const fn schema(&self) -> &ToolSchema {
        &self.schema
    }
}

impl RunnableCore for RunnableTool {
    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        self.inner.invoke(input, config)
    }

    fn name(&self) -> &str {
        &self.schema.name
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runnables::passthrough::RunnablePassthrough;

    #[tokio::test]
    async fn test_runnable_tool_delegates() {
        let schema = ToolSchema {
            name: "my_tool".into(),
            description: "A test tool".into(),
            parameters: serde_json::json!({"type": "object"}),
        };
        let tool = RunnableTool::new(Box::new(RunnablePassthrough), schema);

        assert_eq!(tool.name(), "my_tool");
        assert_eq!(tool.schema().description, "A test tool");

        let result = tool.invoke(Value::from(42), None).await.unwrap();
        assert_eq!(result, Value::from(42));
    }
}

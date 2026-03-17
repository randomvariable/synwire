//! Bidirectional MCP ↔ Synwire tool conversion.

use std::sync::Arc;

use serde_json::Value;
use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::error::SynwireError;
use synwire_core::mcp::traits::{McpToolDescriptor, McpTransport};
use synwire_core::tools::{Tool, ToolOutput, ToolSchema, validate_tool_name};

use crate::convert::content::convert_mcp_response_to_tool_output;
use crate::error::McpAdapterError;

// ---------------------------------------------------------------------------
// MCP → Synwire conversion (T276)
// ---------------------------------------------------------------------------

/// Converts an MCP tool descriptor into a boxed Synwire [`Tool`].
///
/// The generated tool:
/// - Uses the MCP tool name (with optional prefix) as its name.
/// - Forwards invocations to `transport.call_tool`.
/// - Returns `(content, artifact)` in a [`ToolOutput`].
///
/// # Errors
///
/// Returns [`McpAdapterError::Transport`] if the tool name is invalid after
/// prefix application.
pub fn convert_mcp_tool_to_synwire_tool(
    descriptor: &McpToolDescriptor,
    transport: Arc<dyn McpTransport>,
    name_prefix: Option<&str>,
) -> Result<Box<dyn Tool>, McpAdapterError> {
    let exposed_name = name_prefix.map_or_else(
        || descriptor.name.clone(),
        |prefix| format!("{prefix}/{}", descriptor.name),
    );

    // Validate the exposed name (sanitised prefix may still produce invalid chars)
    // MCP tool names are validated on the server; we only reject truly invalid names.
    // The `/` in prefixed names is permitted by our routing layer, so we validate
    // only the base name.
    validate_tool_name(&descriptor.name).map_err(|e| McpAdapterError::Transport {
        message: format!("Invalid MCP tool name '{}': {}", descriptor.name, e),
    })?;

    let original_name = descriptor.name.clone();
    let description = descriptor.description.clone();
    let schema = ToolSchema {
        name: exposed_name.clone(),
        description: description.clone(),
        parameters: descriptor.input_schema.clone(),
    };

    let tool = McpBackedTool {
        name: exposed_name,
        description,
        schema,
        original_name,
        transport,
    };

    Ok(Box::new(tool))
}

/// Synwire-side Synwire → MCP conversion (T277).
///
/// Converts a Synwire tool's schema to the MCP tool definition JSON object.
///
/// # Errors
///
/// Returns [`McpAdapterError::SchemaValidation`] if the schema contains
/// fields that would be injected (e.g. reserved MCP fields).
pub fn to_mcp_tool(tool: &dyn Tool) -> Result<Value, McpAdapterError> {
    let schema = tool.schema();

    // Reject schemas that contain known injected argument names.
    let reserved = ["_mcp_session", "_mcp_server", "_mcp_request_id"];
    if let Some(props) = schema
        .parameters
        .get("properties")
        .and_then(|p| p.as_object())
    {
        for reserved_name in &reserved {
            if props.contains_key(*reserved_name) {
                return Err(McpAdapterError::SchemaValidation {
                    tool: tool.name().to_owned(),
                    reason: format!("schema contains reserved argument name '{reserved_name}'"),
                });
            }
        }
    }

    Ok(serde_json::json!({
        "name": tool.name(),
        "description": tool.description(),
        "inputSchema": schema.parameters,
    }))
}

// ---------------------------------------------------------------------------
// McpBackedTool — a Synwire Tool backed by a remote MCP tool
// ---------------------------------------------------------------------------

struct McpBackedTool {
    name: String,
    description: String,
    schema: ToolSchema,
    /// Original tool name on the MCP server (without prefix).
    original_name: String,
    transport: Arc<dyn McpTransport>,
}

impl std::fmt::Debug for McpBackedTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpBackedTool")
            .field("name", &self.name)
            .field("original_name", &self.original_name)
            .finish_non_exhaustive()
    }
}

impl Tool for McpBackedTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let raw = self
                .transport
                .call_tool(&self.original_name, input)
                .await
                .map_err(agent_error_to_synwire)?;

            convert_mcp_response_to_tool_output(raw).map_err(mcp_error_to_synwire)
        })
    }
}

fn agent_error_to_synwire(e: AgentError) -> SynwireError {
    SynwireError::Other(Box::new(e))
}

fn mcp_error_to_synwire(e: McpAdapterError) -> SynwireError {
    SynwireError::Other(Box::new(e))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use synwire_core::mcp::traits::{McpConnectionState, McpServerStatus};

    struct MockTransport {
        tools: Vec<McpToolDescriptor>,
        response: Value,
    }

    impl McpTransport for MockTransport {
        fn connect(&self) -> synwire_core::BoxFuture<'_, Result<(), AgentError>> {
            Box::pin(async { Ok(()) })
        }
        fn reconnect(&self) -> synwire_core::BoxFuture<'_, Result<(), AgentError>> {
            Box::pin(async { Ok(()) })
        }
        fn status(&self) -> synwire_core::BoxFuture<'_, McpServerStatus> {
            Box::pin(async {
                McpServerStatus {
                    name: "mock".into(),
                    state: McpConnectionState::Connected,
                    calls_succeeded: 0,
                    calls_failed: 0,
                    enabled: true,
                }
            })
        }
        fn list_tools(
            &self,
        ) -> synwire_core::BoxFuture<'_, Result<Vec<McpToolDescriptor>, AgentError>> {
            let tools = self.tools.clone();
            Box::pin(async move { Ok(tools) })
        }
        fn call_tool(
            &self,
            _tool_name: &str,
            _arguments: Value,
        ) -> synwire_core::BoxFuture<'_, Result<Value, AgentError>> {
            let resp = self.response.clone();
            Box::pin(async move { Ok(resp) })
        }
        fn disconnect(&self) -> synwire_core::BoxFuture<'_, Result<(), AgentError>> {
            Box::pin(async { Ok(()) })
        }
    }

    fn make_descriptor(name: &str) -> McpToolDescriptor {
        McpToolDescriptor {
            name: name.to_owned(),
            description: format!("A {name} tool"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }),
        }
    }

    #[tokio::test]
    async fn convert_mcp_to_synwire_and_invoke() {
        let transport = Arc::new(MockTransport {
            tools: vec![make_descriptor("search")],
            response: serde_json::json!({
                "content": [{ "type": "text", "text": "result" }],
                "isError": false
            }),
        });

        let tool =
            convert_mcp_tool_to_synwire_tool(&make_descriptor("search"), transport, None).unwrap();
        assert_eq!(tool.name(), "search");

        let output = tool
            .invoke(serde_json::json!({ "query": "hello" }))
            .await
            .unwrap();
        assert_eq!(output.content, "result");
    }

    #[tokio::test]
    async fn prefixed_tool_name() {
        let transport = Arc::new(MockTransport {
            tools: vec![make_descriptor("search")],
            response: serde_json::json!({
                "content": [{ "type": "text", "text": "ok" }],
                "isError": false
            }),
        });

        let tool = convert_mcp_tool_to_synwire_tool(
            &make_descriptor("search"),
            transport,
            Some("myserver"),
        )
        .unwrap();
        assert_eq!(tool.name(), "myserver/search");
    }

    #[test]
    fn to_mcp_tool_basic() {
        use synwire_core::tools::StructuredTool;
        let synwire_tool = StructuredTool::builder()
            .name("mytool")
            .description("A test tool")
            .schema(ToolSchema {
                name: "mytool".into(),
                description: "A test tool".into(),
                parameters: serde_json::json!({"type": "object"}),
            })
            .func(|_| Box::pin(async { Ok(ToolOutput::default()) }))
            .build()
            .unwrap();

        let mcp_def = to_mcp_tool(&synwire_tool).unwrap();
        assert_eq!(mcp_def["name"], "mytool");
        assert_eq!(mcp_def["description"], "A test tool");
    }

    #[test]
    fn to_mcp_tool_rejects_reserved_args() {
        use synwire_core::tools::StructuredTool;
        let synwire_tool = StructuredTool::builder()
            .name("badtool")
            .description("Has reserved args")
            .schema(ToolSchema {
                name: "badtool".into(),
                description: "Has reserved args".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": { "_mcp_session": { "type": "string" } }
                }),
            })
            .func(|_| Box::pin(async { Ok(ToolOutput::default()) }))
            .build()
            .unwrap();

        let result = to_mcp_tool(&synwire_tool);
        assert!(result.is_err());
    }
}

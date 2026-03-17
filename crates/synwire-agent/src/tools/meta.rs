//! `meta.*` tool provider for tool discovery and introspection.
//!
//! These opt-in tools allow the agent to search for and list available tools at
//! runtime, supporting progressive disclosure of large tool suites.

use synwire_core::error::SynwireError;
use synwire_core::tools::{
    StaticToolProvider, StructuredTool, Tool, ToolOutput, ToolProvider, ToolSchema,
};

/// Build a tool provider for `meta.*` tools (opt-in).
///
/// The returned provider includes:
/// - `meta.search` (semantic tool search via `ToolSearchIndex`)
/// - `meta.list` (list available tools with optional namespace filter)
///
/// # Errors
///
/// Returns [`SynwireError`] if any tool fails validation.
pub fn meta_tool_provider() -> Result<Box<dyn ToolProvider>, SynwireError> {
    let tools: Vec<Box<dyn Tool>> =
        vec![Box::new(build_meta_search()?), Box::new(build_meta_list()?)];
    Ok(Box::new(StaticToolProvider::new(tools)))
}

/// Create a stub tool that returns a "not configured" message.
fn stub_response(tool_name: &str) -> ToolOutput {
    ToolOutput {
        content: format!(
            "{tool_name}: not configured. This tool requires a ToolSearchIndex. \
             Configure the search index to enable this tool."
        ),
        ..Default::default()
    }
}

fn build_meta_search() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("meta.search")
        .description(
            "Search for available tools by intent. Uses embedding-based retrieval \
             from the ToolSearchIndex to find the most relevant tools for a task.",
        )
        .schema(ToolSchema {
            name: "meta.search".into(),
            description: "Search for tools by intent".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language description of what you want to do"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of tools to return (default: 5)"
                    },
                    "namespace": {
                        "type": "string",
                        "description": "Restrict search to a namespace (e.g. 'code', 'fs', 'debug')"
                    }
                },
                "required": ["query"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("meta.search")) }))
        .build()
}

fn build_meta_list() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("meta.list")
        .description(
            "List all available tools, optionally filtered by namespace prefix. \
             Returns tool names and short descriptions.",
        )
        .schema(ToolSchema {
            name: "meta.list".into(),
            description: "List available tools".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "namespace": {
                        "type": "string",
                        "description": "Filter by namespace prefix (e.g. 'code', 'fs')"
                    }
                },
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("meta.list")) }))
        .build()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn meta_provider_discovers_all_tools() {
        let provider = meta_tool_provider().unwrap();
        let tools = provider.discover_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
    }

    #[tokio::test]
    async fn meta_provider_get_by_name() {
        let provider = meta_tool_provider().unwrap();
        let tool = provider.get_tool("meta.search").await.unwrap();
        assert!(tool.is_some());
        let tool = provider.get_tool("meta.list").await.unwrap();
        assert!(tool.is_some());
        let missing = provider.get_tool("meta.nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn stub_tools_return_not_configured() {
        let provider = meta_tool_provider().unwrap();
        let tool = provider.get_tool("meta.search").await.unwrap().unwrap();
        let output = tool
            .invoke(serde_json::json!({"query": "find files"}))
            .await
            .unwrap();
        assert!(output.content.contains("not configured"));
    }
}

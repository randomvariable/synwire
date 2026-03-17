//! `index.*` tool provider for semantic indexing operations.
//!
//! These tools control the indexing pipeline (walk, chunk, embed, store)
//! and provide document-level search.

use synwire_core::error::SynwireError;
use synwire_core::tools::{
    StaticToolProvider, StructuredTool, Tool, ToolOutput, ToolProvider, ToolSchema,
};

/// Build a tool provider for `index.*` tools.
///
/// The returned provider includes:
/// - `index.build` (trigger indexing pipeline)
/// - `index.status` (check indexing progress)
/// - `index.search_docs` (semantic document search)
/// - `index.search_docs_hybrid` (combined semantic + keyword document search)
///
/// # Errors
///
/// Returns [`SynwireError`] if any tool fails validation.
pub fn index_tool_provider() -> Result<Box<dyn ToolProvider>, SynwireError> {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(build_index_build()?),
        Box::new(build_index_status()?),
        Box::new(build_index_search_docs()?),
        Box::new(build_index_search_docs_hybrid()?),
    ];
    Ok(Box::new(StaticToolProvider::new(tools)))
}

/// Create a stub tool that returns a "not configured" message.
fn stub_response(tool_name: &str) -> ToolOutput {
    ToolOutput {
        content: format!(
            "{tool_name}: not configured. This tool requires the indexing daemon. \
             Configure the daemon to enable this tool."
        ),
        ..Default::default()
    }
}

fn build_index_build() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("index.build")
        .description(
            "Trigger or resume the indexing pipeline for the current project. \
             Walks files, chunks with tree-sitter, embeds, and stores vectors.",
        )
        .schema(ToolSchema {
            name: "index.build".into(),
            description: "Trigger the indexing pipeline".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "force": {
                        "type": "boolean",
                        "description": "Force full re-index (default: false, incremental)"
                    },
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Restrict indexing to specific paths"
                    }
                },
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("index.build")) }))
        .build()
}

fn build_index_status() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("index.status")
        .description(
            "Check the current indexing progress and statistics: files indexed, \
             chunks stored, last update time, and any errors.",
        )
        .schema(ToolSchema {
            name: "index.status".into(),
            description: "Check indexing progress and statistics".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("index.status")) }))
        .build()
}

fn build_index_search_docs() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("index.search_docs")
        .description(
            "Search indexed documents using semantic similarity (embedding-based). \
             Returns ranked document chunks with file paths and relevance scores.",
        )
        .schema(ToolSchema {
            name: "index.search_docs".into(),
            description: "Semantic document search".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 10)"
                    },
                    "file_filter": {
                        "type": "string",
                        "description": "Glob pattern to restrict search to matching files"
                    }
                },
                "required": ["query"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("index.search_docs")) }))
        .build()
}

fn build_index_search_docs_hybrid() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("index.search_docs_hybrid")
        .description(
            "Search indexed documents using combined semantic and keyword matching. \
             Merges embedding similarity with BM25 text relevance.",
        )
        .schema(ToolSchema {
            name: "index.search_docs_hybrid".into(),
            description: "Hybrid semantic + keyword document search".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 10)"
                    },
                    "file_filter": {
                        "type": "string",
                        "description": "Glob pattern to restrict search to matching files"
                    },
                    "semantic_weight": {
                        "type": "number",
                        "description": "Weight for semantic score (0.0-1.0, default: 0.6)"
                    }
                },
                "required": ["query"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("index.search_docs_hybrid")) }))
        .build()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn index_provider_discovers_all_tools() {
        let provider = index_tool_provider().unwrap();
        let tools = provider.discover_tools().await.unwrap();
        assert_eq!(tools.len(), 4);
    }

    #[tokio::test]
    async fn index_provider_get_by_name() {
        let provider = index_tool_provider().unwrap();
        let tool = provider.get_tool("index.build").await.unwrap();
        assert!(tool.is_some());
        let missing = provider.get_tool("index.nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn stub_tools_return_not_configured() {
        let provider = index_tool_provider().unwrap();
        let tool = provider.get_tool("index.status").await.unwrap().unwrap();
        let output = tool.invoke(serde_json::json!({})).await.unwrap();
        assert!(output.content.contains("not configured"));
    }
}

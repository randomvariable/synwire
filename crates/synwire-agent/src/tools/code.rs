//! `code.*` tool provider for semantic code navigation and search.
//!
//! The tools in this module provide multi-backend dispatch for code
//! intelligence operations. When no backend is configured, each tool returns a
//! descriptive message explaining which dependencies are required.
//!
//! The actual backend dispatch (daemon proxy, LSP client) is injected by the
//! consumer (e.g. the MCP server) at startup.

use synwire_core::error::SynwireError;
use synwire_core::tools::{
    StaticToolProvider, StructuredTool, Tool, ToolOutput, ToolProvider, ToolSchema,
};

/// Configuration controlling which `code.*` backends are available.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct CodeToolConfig {
    /// If set, daemon-backed tools are available.
    pub daemon_available: bool,
    /// If set, LSP-backed tools are available.
    pub lsp_available: bool,
}

/// Build a tool provider containing all `code.*` tools.
///
/// The returned provider includes:
/// - `code.search` (semantic/graph/community modes)
/// - `code.search_hybrid` (combined semantic + keyword search)
/// - `code.definition` (LSP-first, graph fallback)
/// - `code.references` (LSP -> xref -> graph fallback)
/// - `code.symbols` (LSP with skeleton fallback)
/// - `code.type_info` (LSP hover)
/// - `code.dependencies` (package/module dependency graph)
/// - `code.community_members` (community detection clusters)
/// - `code.trace_dataflow` (data flow analysis)
/// - `code.trace_callers` (call graph traversal)
/// - `code.fault_localize` (SBFL-based fault localization)
///
/// # Errors
///
/// Returns [`SynwireError`] if any tool fails validation.
pub fn code_tool_provider() -> Result<Box<dyn ToolProvider>, SynwireError> {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(build_code_search()?),
        Box::new(build_code_search_hybrid()?),
        Box::new(build_code_definition()?),
        Box::new(build_code_references()?),
        Box::new(build_code_symbols()?),
        Box::new(build_code_type_info()?),
        Box::new(build_code_dependencies()?),
        Box::new(build_code_community_members()?),
        Box::new(build_code_trace_dataflow()?),
        Box::new(build_code_trace_callers()?),
        Box::new(build_code_fault_localize()?),
    ];
    Ok(Box::new(StaticToolProvider::new(tools)))
}

/// Create a stub tool that returns a "not configured" message.
///
/// This is the default behaviour until the consumer injects a real backend.
fn stub_response(tool_name: &str) -> ToolOutput {
    ToolOutput {
        content: format!(
            "{tool_name}: not configured. This tool requires a daemon or LSP backend. \
             Configure the appropriate backend to enable this tool."
        ),
        ..Default::default()
    }
}

fn build_code_search() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.search")
        .description(
            "Search code semantically using embeddings, call graphs, or community clusters. \
             Supports modes: semantic, graph, community.",
        )
        .schema(ToolSchema {
            name: "code.search".into(),
            description: "Search code semantically".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["semantic", "graph", "community"],
                        "description": "Search mode (default: semantic)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 10)"
                    }
                },
                "required": ["query"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.search")) }))
        .build()
}

fn build_code_search_hybrid() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.search_hybrid")
        .description(
            "Combined semantic and keyword search across the codebase. \
             Merges embedding similarity with BM25 text matching.",
        )
        .schema(ToolSchema {
            name: "code.search_hybrid".into(),
            description: "Hybrid semantic + keyword code search".into(),
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
                    }
                },
                "required": ["query"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.search_hybrid")) }))
        .build()
}

fn build_code_definition() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.definition")
        .description(
            "Go to definition of a symbol. Uses LSP when available, \
             falls back to call graph data.",
        )
        .schema(ToolSchema {
            name: "code.definition".into(),
            description: "Find the definition of a symbol".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path containing the symbol"
                    },
                    "line": {
                        "type": "integer",
                        "description": "1-based line number"
                    },
                    "column": {
                        "type": "integer",
                        "description": "1-based column number"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "Symbol name (used for graph fallback)"
                    }
                },
                "required": ["file", "line", "column"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.definition")) }))
        .build()
}

fn build_code_references() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.references")
        .description(
            "Find all references to a symbol. Tries LSP, cross-reference index, \
             then call graph in order of availability.",
        )
        .schema(ToolSchema {
            name: "code.references".into(),
            description: "Find all references to a symbol".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path containing the symbol"
                    },
                    "line": {
                        "type": "integer",
                        "description": "1-based line number"
                    },
                    "column": {
                        "type": "integer",
                        "description": "1-based column number"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "Symbol name (used for index/graph fallback)"
                    }
                },
                "required": ["file", "line", "column"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.references")) }))
        .build()
}

fn build_code_symbols() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.symbols")
        .description(
            "List symbols in a file or workspace. Uses LSP document/workspace symbols \
             when available, falls back to tree-sitter skeleton extraction.",
        )
        .schema(ToolSchema {
            name: "code.symbols".into(),
            description: "List symbols in a file or workspace".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path (omit for workspace-wide search)"
                    },
                    "query": {
                        "type": "string",
                        "description": "Filter symbols by name pattern"
                    }
                },
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.symbols")) }))
        .build()
}

fn build_code_type_info() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.type_info")
        .description(
            "Get type information and documentation for a symbol at a given position. \
             Backed by LSP hover.",
        )
        .schema(ToolSchema {
            name: "code.type_info".into(),
            description: "Get type info for a symbol via LSP hover".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path"
                    },
                    "line": {
                        "type": "integer",
                        "description": "1-based line number"
                    },
                    "column": {
                        "type": "integer",
                        "description": "1-based column number"
                    }
                },
                "required": ["file", "line", "column"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.type_info")) }))
        .build()
}

fn build_code_dependencies() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.dependencies")
        .description("List package or module dependencies for a file or the project root.")
        .schema(ToolSchema {
            name: "code.dependencies".into(),
            description: "List package/module dependencies".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path (omit for project-level dependencies)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum dependency depth (default: 1)"
                    }
                },
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.dependencies")) }))
        .build()
}

fn build_code_community_members() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.community_members")
        .description(
            "List symbols belonging to the same community cluster as the given symbol. \
             Requires community detection index (hit-leiden).",
        )
        .schema(ToolSchema {
            name: "code.community_members".into(),
            description: "List symbols in the same community cluster".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Fully qualified symbol name"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of members (default: 20)"
                    }
                },
                "required": ["symbol"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.community_members")) }))
        .build()
}

fn build_code_trace_dataflow() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.trace_dataflow")
        .description("Trace data flow forwards or backwards from a variable or expression.")
        .schema(ToolSchema {
            name: "code.trace_dataflow".into(),
            description: "Trace data flow from a variable".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path"
                    },
                    "line": {
                        "type": "integer",
                        "description": "1-based line number"
                    },
                    "column": {
                        "type": "integer",
                        "description": "1-based column number"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["forward", "backward"],
                        "description": "Trace direction (default: forward)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum trace depth (default: 5)"
                    }
                },
                "required": ["file", "line", "column"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.trace_dataflow")) }))
        .build()
}

fn build_code_trace_callers() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.trace_callers")
        .description(
            "Trace the call graph upward from a function to find all callers, \
             transitively up to a configurable depth.",
        )
        .schema(ToolSchema {
            name: "code.trace_callers".into(),
            description: "Trace callers of a function".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Fully qualified function name"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum caller depth (default: 3)"
                    }
                },
                "required": ["symbol"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.trace_callers")) }))
        .build()
}

fn build_code_fault_localize() -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("code.fault_localize")
        .description(
            "Rank files and functions by suspiciousness using spectrum-based fault \
             localization (SBFL). Requires test coverage data.",
        )
        .schema(ToolSchema {
            name: "code.fault_localize".into(),
            description: "SBFL fault localization".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "failing_tests": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of failing test identifiers"
                    },
                    "formula": {
                        "type": "string",
                        "enum": ["ochiai", "tarantula", "dstar"],
                        "description": "SBFL formula (default: ochiai)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 20)"
                    }
                },
                "required": ["failing_tests"],
                "additionalProperties": false,
            }),
        })
        .func(|_input| Box::pin(async { Ok(stub_response("code.fault_localize")) }))
        .build()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn code_provider_discovers_all_tools() {
        let provider = code_tool_provider().unwrap();
        let tools = provider.discover_tools().await.unwrap();
        assert_eq!(tools.len(), 11);
    }

    #[tokio::test]
    async fn code_provider_get_by_name() {
        let provider = code_tool_provider().unwrap();
        let tool = provider.get_tool("code.search").await.unwrap();
        assert!(tool.is_some());
        let missing = provider.get_tool("code.nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn stub_tools_return_not_configured() {
        let provider = code_tool_provider().unwrap();
        let tool = provider.get_tool("code.definition").await.unwrap().unwrap();
        let output = tool
            .invoke(serde_json::json!({"file": "main.rs", "line": 1, "column": 1}))
            .await
            .unwrap();
        assert!(output.content.contains("not configured"));
    }
}

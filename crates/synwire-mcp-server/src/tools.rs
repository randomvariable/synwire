//! MCP tool definitions for the Synwire MCP server.
//!
//! Each tool is exposed with a JSON Schema definition and an LLM-optimised
//! description explaining when to use it.
//!
//! Tool names use dot-namespaced convention: `<namespace>.<action>` (e.g.
//! `fs.read`, `code.search`, `lsp.hover`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Metadata for a single MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name (dot-namespaced, e.g. `fs.read`, `code.search`).
    pub name: String,
    /// LLM-optimised description explaining purpose and when to use this tool.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: Value,
}

impl McpTool {
    /// Infer a namespace from the tool name by taking the dot-separated prefix.
    ///
    /// For example, `fs.read` -> `"fs"`, `code.search` -> `"code"`.
    pub fn namespace(&self) -> &str {
        self.name.split('.').next().unwrap_or("misc")
    }
}

/// Return all built-in tools available in the MCP server.
///
/// These are registered at startup and available before any project is indexed.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn builtin_tools() -> Vec<McpTool> {
    vec![
        // -----------------------------------------------------------------
        // fs.* -- VFS file operations
        // -----------------------------------------------------------------
        McpTool {
            name: "fs.read".to_owned(),
            description: "Read the full contents of a file. Use when you need to examine a \
                           specific file. Returns plain text."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or project-relative file path" }
                },
                "required": ["path"]
            }),
        },
        McpTool {
            name: "fs.write".to_owned(),
            description: "Write content to a file, creating it if it does not exist. Use for \
                           creating new files or completely replacing existing content."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string", "description": "Full file content to write" }
                },
                "required": ["path", "content"]
            }),
        },
        McpTool {
            name: "fs.edit".to_owned(),
            description: "Make targeted edits to a file by replacing exact text. Prefer over \
                           'fs.write' when modifying a specific section. The old_string must \
                           match exactly."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_string": { "type": "string", "description": "Exact text to replace" },
                    "new_string": { "type": "string", "description": "Replacement text" }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        },
        McpTool {
            name: "fs.grep".to_owned(),
            description: "Search file contents using a regex pattern. Returns matching lines \
                           with file paths and line numbers. Use for finding usages, definitions, \
                           or specific patterns."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regular expression to search for" },
                    "path": { "type": "string", "description": "Directory or file to search (default: project root)" },
                    "case_insensitive": { "type": "boolean" },
                    "context_lines": { "type": "integer", "description": "Lines of context before/after each match" }
                },
                "required": ["pattern"]
            }),
        },
        McpTool {
            name: "fs.glob".to_owned(),
            description: "Find files by name pattern (glob syntax, e.g. '**/*.rs'). Use to \
                           locate files when you know the naming pattern but not the exact path."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Glob pattern (e.g. '**/*.rs')" },
                    "base": { "type": "string", "description": "Base directory for the search" }
                },
                "required": ["pattern"]
            }),
        },
        McpTool {
            name: "fs.tree".to_owned(),
            description: "Show directory structure as a tree. Use to understand project layout \
                           before navigating to specific files."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Root directory path" },
                    "depth": { "type": "integer", "description": "Maximum depth (default: 3)" }
                }
            }),
        },
        McpTool {
            name: "fs.skeleton".to_owned(),
            description: "Show function/method signatures without bodies. Returns a \
                           token-efficient overview of a file's public API. Use before reading \
                           full file content to orient yourself."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to extract skeleton from" }
                },
                "required": ["path"]
            }),
        },
        // -----------------------------------------------------------------
        // index.* -- Indexing operations
        // -----------------------------------------------------------------
        McpTool {
            name: "index.build".to_owned(),
            description: "Index the project for semantic search. Must be called once before \
                           using 'code.search'. Safe to call multiple times -- incremental \
                           updates only changed files."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project root to index (default: configured --project)" }
                }
            }),
        },
        McpTool {
            name: "index.status".to_owned(),
            description: "Check the current indexing status: how many files are indexed, when \
                           last updated, and whether indexing is in progress."
                .to_owned(),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        },
        McpTool {
            name: "index.search_docs".to_owned(),
            description: "Search documentation files semantically using natural language. \
                           Returns the most relevant documentation chunks. Requires \
                           'index.build' first. Filters results to documentation file types \
                           only."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "top_k": { "type": "integer", "description": "Number of results to return (default: 10)" }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "index.search_docs_hybrid".to_owned(),
            description: "Search documentation files using hybrid (semantic + keyword) search. \
                           Combines embedding similarity with BM25 ranking for documentation \
                           files only. Requires 'index.build' first."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "top_k": { "type": "integer", "description": "Number of results to return (default: 10)" }
                },
                "required": ["query"]
            }),
        },
        // -----------------------------------------------------------------
        // code.* -- Code search and analysis
        // -----------------------------------------------------------------
        McpTool {
            name: "code.search".to_owned(),
            description: "Search the codebase using natural language. Supports three modes: \
                           'by_meaning' (embedding similarity), 'by_graph' (ego-graph expansion \
                           around similar symbols), and 'by_community' (graph-clustered \
                           community search). Requires 'index.build' first. Default mode: \
                           'by_meaning'."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "mode": {
                        "type": "string",
                        "enum": ["by_meaning", "by_graph", "by_community"],
                        "description": "Search mode (default: 'by_meaning')"
                    },
                    "top_k": { "type": "integer", "description": "Number of results to return (default: 10)" },
                    "hops": { "type": "integer", "description": "Number of graph hops to expand (only for 'by_graph' mode, default: 2)" }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "code.search_hybrid".to_owned(),
            description: "Search the codebase using hybrid (semantic + keyword) search. \
                           Combines embedding similarity with BM25 ranking. Requires \
                           'index.build' first."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "top_k": { "type": "integer", "description": "Number of results to return (default: 10)" }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "code.definition".to_owned(),
            description: "Jump to the definition of a symbol. When file/line/column are \
                           provided and LSP is available, uses the language server. When only \
                           a symbol name is given, queries the code dependency graph. Returns \
                           file path and line number."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path (for LSP-based lookup)" },
                    "line": { "type": "integer", "description": "Line number, 1-based (for LSP-based lookup)" },
                    "column": { "type": "integer", "description": "Column number, 1-based (for LSP-based lookup)" },
                    "symbol": { "type": "string", "description": "Fully qualified symbol name (for graph-based lookup)" }
                }
            }),
        },
        McpTool {
            name: "code.references".to_owned(),
            description: "Find all references to a symbol. When file/line/column are provided \
                           and LSP is available, uses the language server. Otherwise, queries \
                           the cross-reference index and code dependency graph with incoming \
                           edges."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path (for LSP-based lookup)" },
                    "line": { "type": "integer", "description": "Line number, 1-based (for LSP-based lookup)" },
                    "column": { "type": "integer", "description": "Column number, 1-based (for LSP-based lookup)" },
                    "symbol": { "type": "string", "description": "Fully qualified symbol name (for index-based lookup)" }
                }
            }),
        },
        McpTool {
            name: "code.symbols".to_owned(),
            description: "List all symbols defined in a file (functions, types, variables). \
                           Tries LSP document symbols first if available, then falls back to \
                           local skeleton extraction."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" }
                },
                "required": ["file"]
            }),
        },
        McpTool {
            name: "code.type_info".to_owned(),
            description: "Get hover information (type, documentation) for a symbol at a file \
                           position. Requires --lsp configured."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "Line number (1-based)" },
                    "column": { "type": "integer", "description": "Column number (1-based)" }
                },
                "required": ["file", "line", "column"]
            }),
        },
        McpTool {
            name: "code.dependencies".to_owned(),
            description: "Query the code dependency graph for a symbol. Returns callers, \
                           callees, imports, and inheritance relationships. Requires indexing \
                           first."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Fully qualified symbol name (e.g. 'MyStruct::my_method')" },
                    "depth": { "type": "integer", "description": "Traversal depth (default: 2)" },
                    "direction": { "type": "string", "enum": ["outgoing", "incoming", "both"], "description": "Edge direction (default: both)" }
                },
                "required": ["symbol"]
            }),
        },
        McpTool {
            name: "code.community_members".to_owned(),
            description: "List members (files and symbols) of a specific code community."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "community_id": { "type": "string", "description": "Community identifier" }
                },
                "required": ["community_id"]
            }),
        },
        McpTool {
            name: "code.fault_localize".to_owned(),
            description: "Rank source files by fault likelihood using Spectrum-Based Fault \
                           Localization (SBFL/Ochiai). Provide test coverage data and get files \
                           ranked by suspiciousness score. Use when debugging failing tests to \
                           find the most likely faulty files."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "coverage": { "type": "array", "items": { "type": "object", "properties": {
                        "file": {"type":"string"}, "line": {"type":"integer"},
                        "ef": {"type":"integer","description":"failing tests covering this line"},
                        "nf": {"type":"integer","description":"passing tests covering this line"},
                        "np": {"type":"integer","description":"passing tests NOT covering this line"}
                    }}, "description": "Coverage records for source lines" },
                    "semantic_results": { "type": "array", "items": { "type": "object", "properties": {
                        "file": {"type":"string"}, "score": {"type":"number"}
                    }}, "description": "Optional semantic search scores to fuse with SBFL" },
                    "sbfl_weight": { "type": "number", "description": "Weight for SBFL vs semantic (0.0-1.0, default 0.7)" }
                },
                "required": ["coverage"]
            }),
        },
        McpTool {
            name: "code.trace_dataflow".to_owned(),
            description: "Trace the dataflow of a variable backward through a source file. \
                           Shows where the variable is defined, assigned, and modified. Use to \
                           understand where a value comes from."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path to analyze" },
                    "variable": { "type": "string", "description": "Variable name to trace" },
                    "max_hops": { "type": "integer", "description": "Maximum backward hops (default: 10)" }
                },
                "required": ["file", "variable"]
            }),
        },
        McpTool {
            name: "code.trace_callers".to_owned(),
            description: "Query the dynamic call graph for a symbol. Returns callers (who \
                           calls this?) and callees (what does this call?). Also detects \
                           cycles. Use to understand code dependencies."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to query" },
                    "direction": { "type": "string", "enum": ["callers", "callees", "both"], "description": "Query direction (default: both)" }
                },
                "required": ["symbol"]
            }),
        },
        // -----------------------------------------------------------------
        // vcs.* -- Version control operations
        // -----------------------------------------------------------------
        McpTool {
            name: "vcs.clone".to_owned(),
            description: "Clone a git repository and mount it as a virtual filesystem provider. \
                           Supports GitHub URLs. Optional shallow clone and auto-indexing."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Git repository URL (e.g. 'https://github.com/owner/repo')" },
                    "depth": { "type": "integer", "description": "Shallow clone depth (default: full clone)" },
                    "ref": { "type": "string", "description": "Branch, tag, or commit to checkout" },
                    "index": { "type": "boolean", "description": "Whether to index after cloning (default: false)" }
                },
                "required": ["url"]
            }),
        },
        // -----------------------------------------------------------------
        // lsp.* -- Language server tools (registered unconditionally;
        //          dispatch requires --lsp)
        // -----------------------------------------------------------------
        McpTool {
            name: "lsp.hover".to_owned(),
            description: "Get hover information (type, documentation) for a symbol at a file \
                           position. Requires --lsp configured. Prefer 'code.type_info' which \
                           provides a unified interface."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "Line number (1-based)" },
                    "column": { "type": "integer", "description": "Column number (1-based)" }
                },
                "required": ["file", "line", "column"]
            }),
        },
        McpTool {
            name: "lsp.definition".to_owned(),
            description: "Jump to the definition of a symbol. Returns file path and line \
                           number. Requires --lsp configured. Prefer 'code.definition' which \
                           provides a unified interface with graph fallback."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "Line number (1-based)" },
                    "column": { "type": "integer", "description": "Column number (1-based)" }
                },
                "required": ["file", "line", "column"]
            }),
        },
        McpTool {
            name: "lsp.references".to_owned(),
            description: "Find all references to a symbol. Requires --lsp configured. Prefer \
                           'code.references' which provides a unified interface with index \
                           fallback."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "Line number (1-based)" },
                    "column": { "type": "integer", "description": "Column number (1-based)" }
                },
                "required": ["file", "line", "column"]
            }),
        },
        McpTool {
            name: "lsp.symbols".to_owned(),
            description: "List all symbols defined in a file (functions, types, variables). \
                           Requires --lsp configured. Prefer 'code.symbols' which falls back \
                           to skeleton extraction."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" }
                },
                "required": ["file"]
            }),
        },
        // -----------------------------------------------------------------
        // debug.* -- Debug adapter tools (registered unconditionally;
        //            dispatch requires --dap)
        // -----------------------------------------------------------------
        McpTool {
            name: "debug.breakpoint".to_owned(),
            description: "Set a debug breakpoint at a file and line. Requires --dap configured."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string" },
                    "line": { "type": "integer" }
                },
                "required": ["file", "line"]
            }),
        },
        McpTool {
            name: "debug.evaluate".to_owned(),
            description: "Evaluate an expression in the current debug context. Requires --dap \
                           configured and an active debug session."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string", "description": "Expression to evaluate" }
                },
                "required": ["expression"]
            }),
        },
        // -----------------------------------------------------------------
        // meta.* -- Tool discovery
        // -----------------------------------------------------------------
        McpTool {
            name: "meta.search".to_owned(),
            description: "Discover available tools using a natural language query or browse a \
                           namespace. Use before calling unknown tools to find the right one. \
                           Reduces token usage by ~85% vs listing all tools upfront."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language query to find relevant tools" },
                    "namespace": { "type": "string", "description": "Browse all tools in a namespace (e.g. 'fs', 'index', 'lsp')" },
                    "top_k": { "type": "integer", "description": "Maximum number of results to return (default: 5)" }
                }
            }),
        },
        McpTool {
            name: "meta.list".to_owned(),
            description: "List all available tools grouped by namespace with names and \
                           descriptions only. Use for orientation; use 'meta.search' for \
                           targeted discovery."
                .to_owned(),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        },
    ]
}

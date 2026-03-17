//! MCP server implementation using stdio JSON-RPC transport.
//!
//! The server reads newline-delimited JSON-RPC 2.0 messages from stdin and
//! writes responses to stdout.  All diagnostic output goes to stderr (never
//! stdout, which is reserved for the MCP protocol).
//!
//! ## Protocol
//!
//! Implements the MCP spec:
//! - `initialize` → capabilities response
//! - `tools/list` → array of tool definitions
//! - `tools/call` → invoke a tool and return its result

use crate::proxy::{self, DaemonProxy};
use crate::sampling::McpSampling;
use crate::tools::{McpTool, builtin_tools};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use synwire_core::SamplingProvider as _;
use synwire_core::tools::{ToolSearchArgs, ToolSearchIndex, run_tool_list, run_tool_search};
use synwire_storage::{StorageLayout, WorktreeId};
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

/// Incoming JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
struct Request {
    /// Always `"2.0"`.
    #[allow(dead_code)]
    jsonrpc: String,
    /// Request ID (may be integer, string, or null for notifications).
    id: Option<Value>,
    /// Method name.
    method: String,
    /// Optional parameters.
    #[serde(default)]
    params: Value,
}

/// Outgoing JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl Response {
    const fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Server state
// ---------------------------------------------------------------------------

/// Runtime configuration resolved from CLI + config file.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Project root directory (if configured).
    pub project: Option<PathBuf>,
    /// Product name for storage scoping.
    pub product_name: String,
    /// Embedding model identifier used for tool search and semantic indexing.
    #[allow(dead_code)]
    pub embedding_model: String,
    /// LSP command (e.g. `"rust-analyzer"`). When set, LSP tools dispatch to
    /// a language server process managed by the `synwire-lsp` crate.
    pub lsp: Option<String>,
    /// DAP command (e.g. `"lldb-dap"`). When set, DAP tools dispatch to a
    /// debug adapter process managed by the `synwire-dap` crate.
    pub dap: Option<String>,
}

/// Live MCP server state.
pub struct McpServer {
    options: ServerOptions,
    layout: StorageLayout,
    tools: HashMap<String, McpTool>,
    /// Progressive tool discovery index.
    tool_search_index: RwLock<ToolSearchIndex>,
    /// Proxy for forwarding remote tool calls to the daemon.
    daemon_proxy: DaemonProxy,
    /// MCP sampling provider for tool-internal LLM access.
    ///
    /// Currently always disabled. Full bidirectional MCP transport
    /// (`sampling/createMessage`) is a future phase. Stored here so the
    /// provider is available for injection once transport lands.
    _sampling: McpSampling,
    /// LSP client for language server tool dispatch.
    ///
    /// Initialised lazily on first LSP tool call when `--lsp` is configured
    /// and the `lsp` feature is enabled. Requires a tokio runtime, which is
    /// created per-request in the synchronous `dispatch_tool` path.
    #[cfg(feature = "lsp")]
    lsp_client: RwLock<Option<std::sync::Arc<synwire_lsp::client::LspClient>>>,
    /// DAP client for debug adapter tool dispatch.
    ///
    /// Initialised lazily on first DAP tool call when `--dap` is configured
    /// and the `dap` feature is enabled.
    #[cfg(feature = "dap")]
    dap_client: RwLock<Option<std::sync::Arc<synwire_dap::DapClient>>>,
}

impl McpServer {
    /// Create and initialise a new MCP server.
    ///
    /// # Errors
    ///
    /// Returns an error if the `StorageLayout` cannot be initialised.
    pub fn new(options: ServerOptions) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let layout = StorageLayout::new(&options.product_name)?;

        // Multi-instance safety: StorageLayout uses SQLite WAL mode for all
        // databases, and LanceDB handles concurrent access natively. Multiple
        // MCP server instances can safely share the same data directories.

        // Ensure essential directories exist.
        let _ = layout.ensure_dir(layout.data_home());
        let _ = layout.ensure_dir(&layout.logs_dir());

        if let Some(ref project) = options.project
            && let Ok(wid) = WorktreeId::for_path(project)
        {
            info!(
                worktree = %wid.display_name,
                index_cache = %layout.index_cache(&wid).display(),
                "Project configured"
            );
        }

        // Register built-in tools.
        let mut tools = HashMap::new();
        for tool in builtin_tools() {
            let _ = tools.insert(tool.name.clone(), tool);
        }

        // T214: Auto-discover agent skills and register each as an MCP tool.
        let skills_dir = layout.skills_dir();
        if skills_dir.exists() {
            match scan_skills_sync(&skills_dir) {
                Ok(skill_tools) => {
                    info!(
                        skills = skill_tools.len(),
                        dir = %skills_dir.display(),
                        "Agent skills discovered"
                    );
                    for skill_tool in skill_tools {
                        let _ = tools.insert(skill_tool.name.clone(), skill_tool);
                    }
                }
                Err(e) => {
                    warn!(dir = %skills_dir.display(), error = %e, "Failed to scan skills directory");
                }
            }
        }

        // T212: Log when LSP integration is enabled.
        if options.lsp.is_some() {
            info!(
                "LSP tools enabled -- lsp.hover, lsp.definition, \
                 lsp.references, lsp.symbols available"
            );
        }

        // T213: Log when DAP integration is enabled.
        if options.dap.is_some() {
            info!("DAP tools enabled -- debug.breakpoint, debug.evaluate available");
        }

        // Build the tool search index from registered tools.
        let mut tool_search_index = ToolSearchIndex::new();
        for tool in tools.values() {
            let schema_str = serde_json::to_string(&tool.input_schema).ok();
            tool_search_index.register(
                &tool.name,
                tool.namespace(),
                &tool.description,
                &[],
                schema_str.as_deref(),
            );
        }

        info!(
            product = %options.product_name,
            tools = tools.len(),
            registry_hash = %tool_search_index.registry_hash(),
            "MCP server ready"
        );

        // Initialise the daemon proxy.  Try to start the daemon if it is not
        // already running — log a warning but do not fail startup if it cannot
        // be reached (local-only tools still work).
        let daemon_proxy = DaemonProxy::new(layout.daemon_socket());
        if let Err(e) = DaemonProxy::ensure_daemon_running(&layout) {
            warn!(error = %e, "Daemon not available — remote tools will return errors");
        }

        // Create the MCP sampling provider.  Currently always disabled —
        // full bidirectional transport is a future phase.
        let sampling = McpSampling::new();
        info!(
            sampling_available = sampling.is_available(),
            "MCP sampling provider initialised (bidirectional transport pending)"
        );

        Ok(Self {
            options,
            layout,
            tools,
            tool_search_index: RwLock::new(tool_search_index),
            daemon_proxy,
            _sampling: sampling,
            #[cfg(feature = "lsp")]
            lsp_client: RwLock::new(None),
            #[cfg(feature = "dap")]
            dap_client: RwLock::new(None),
        })
    }

    /// Serve the MCP protocol over stdin/stdout until EOF.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if stdin/stdout cannot be accessed.
    pub fn serve(&self) -> std::io::Result<()> {
        use std::io::{BufRead, Write};

        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        info!("Listening for MCP requests on stdin");

        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            debug!(request = %line, "Received MCP request");

            let response = match serde_json::from_str::<Request>(&line) {
                Ok(req) => self.handle_request(req),
                Err(e) => Response::err(Value::Null, -32700, format!("Parse error: {e}")),
            };

            let mut json = serde_json::to_string(&response).unwrap_or_else(|_| {
                r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"internal error"}}"#
                    .to_owned()
            });
            json.push('\n');
            stdout.write_all(json.as_bytes())?;
            stdout.flush()?;
        }

        info!("stdin closed, shutting down");
        Ok(())
    }

    fn handle_request(&self, req: Request) -> Response {
        let id = req.id.clone().unwrap_or(Value::Null);

        match req.method.as_str() {
            "initialize" => Self::handle_initialize(id, req.params),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, &req.params),
            "ping" => Response::ok(id, serde_json::json!({})),
            method => {
                warn!(method, "Unknown MCP method");
                Response::err(id, -32601, format!("Method not found: {method}"))
            }
        }
    }

    fn handle_initialize(id: Value, _params: Value) -> Response {
        Response::ok(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "synwire-mcp-server",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Value) -> Response {
        // Return a compact listing (name + description only) to minimise token
        // consumption. Clients can use `tool_search` to retrieve full schemas.
        let compact = self
            .tool_search_index
            .read()
            .map(|idx| idx.list_compact())
            .unwrap_or_default();

        let tools: Vec<Value> = compact
            .iter()
            .map(|(name, desc)| {
                serde_json::json!({
                    "name": name,
                    "description": desc,
                    "inputSchema": { "type": "object", "properties": {} }
                })
            })
            .collect();
        Response::ok(id, serde_json::json!({ "tools": tools }))
    }

    fn handle_tools_call(&self, id: Value, params: &Value) -> Response {
        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_owned(),
            None => return Response::err(id, -32602, "Missing 'name' parameter"),
        };
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::default()));

        if !self.tools.contains_key(&tool_name) {
            return Response::err(id, -32602, format!("Unknown tool: {tool_name}"));
        }

        match self.dispatch_tool(&tool_name, &arguments) {
            Ok(content) => {
                // Record successful tool invocations for adaptive search scoring.
                if let Ok(mut idx) = self.tool_search_index.write() {
                    idx.record_success(&tool_name, &tool_name);
                }
                Response::ok(
                    id,
                    serde_json::json!({
                        "content": [{ "type": "text", "text": content }]
                    }),
                )
            }
            Err(e) => {
                error!(tool = %tool_name, error = %e, "Tool call failed");
                Response::err(id, -32603, e.to_string())
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn dispatch_tool(&self, name: &str, args: &Value) -> Result<String, ToolError> {
        let project = self
            .options
            .project
            .as_deref()
            .ok_or_else(|| ToolError::NotConfigured("No --project configured".to_owned()));

        match name {
            "fs.read" => {
                let path = require_str(args, "path")?;
                let full = resolve_path(project?, path);
                std::fs::read_to_string(&full).map_err(|e| ToolError::Io(e.to_string()))
            }
            "fs.tree" => {
                let project = project?;
                let base = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| project.to_path_buf(), |p| project.join(p));
                let depth = args.get("depth").and_then(Value::as_u64).unwrap_or(3);
                Ok(dir_tree(&base, usize::try_from(depth).unwrap_or(3), 0))
            }
            "fs.glob" => {
                let pattern = require_str(args, "pattern")?;
                let base = project?;
                let entries = glob_files(base, pattern)?;
                Ok(entries.join("\n"))
            }
            "index.status" => {
                let project = project?;
                let wid =
                    WorktreeId::for_path(project).map_err(|e| ToolError::Other(e.to_string()))?;
                let cache = self.layout.index_cache(&wid);
                if cache.exists() {
                    Ok(format!("Index cache: {}\nStatus: present", cache.display()))
                } else {
                    Ok("Index not yet built. Run 'index.build' tool first.".to_owned())
                }
            }
            "meta.list" => {
                let output = self.tool_search_index.read().map_or_else(
                    |_| "Tool index unavailable.".to_owned(),
                    |idx| run_tool_list(&idx),
                );
                Ok(output)
            }
            "meta.search" => {
                let mut search_args = ToolSearchArgs::new();
                if let Some(q) = args.get("query").and_then(|v| v.as_str()) {
                    search_args = search_args.with_query(q);
                }
                if let Some(ns) = args.get("namespace").and_then(|v| v.as_str()) {
                    search_args = search_args.with_namespace(ns);
                }
                if let Some(k) = args.get("top_k").and_then(Value::as_u64) {
                    search_args = search_args.with_top_k(usize::try_from(k).unwrap_or(10));
                }
                let output = self.tool_search_index.write().map_or_else(
                    |_| "Tool index unavailable.".to_owned(),
                    |mut idx| run_tool_search(&mut idx, &search_args),
                );
                Ok(output)
            }
            "fs.write" => {
                let path = require_str(args, "path")?;
                let content = require_str(args, "content")?;
                let full = resolve_path(project?, path);
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| ToolError::Io(e.to_string()))?;
                }
                std::fs::write(&full, content).map_err(|e| ToolError::Io(e.to_string()))?;
                Ok(format!(
                    "Wrote {} bytes to {}",
                    content.len(),
                    full.display()
                ))
            }
            "fs.edit" => {
                let path = require_str(args, "path")?;
                let old_string = require_str(args, "old_string")?;
                let new_string = require_str(args, "new_string")?;
                let full = resolve_path(project?, path);
                let content =
                    std::fs::read_to_string(&full).map_err(|e| ToolError::Io(e.to_string()))?;
                if !content.contains(old_string) {
                    return Err(ToolError::Other(format!(
                        "old_string not found in {}",
                        full.display()
                    )));
                }
                let new_content = content.replacen(old_string, new_string, 1);
                std::fs::write(&full, &new_content).map_err(|e| ToolError::Io(e.to_string()))?;
                Ok(format!("Edited {}", full.display()))
            }
            "fs.grep" => {
                let pattern = require_str(args, "pattern")?;
                let project_dir = project?;
                let base = args.get("path").and_then(|v| v.as_str()).map_or_else(
                    || project_dir.to_path_buf(),
                    |p| resolve_path(project_dir, p),
                );
                let case_insensitive = args
                    .get("case_insensitive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let context = args
                    .get("context_lines")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let regex_flag = if case_insensitive { "(?i)" } else { "" };
                let re = regex::Regex::new(&format!("{regex_flag}{pattern}"))
                    .map_err(|e| ToolError::Other(format!("Invalid regex: {e}")))?;
                let mut output = String::new();
                grep_recursive(
                    &base,
                    &re,
                    usize::try_from(context).unwrap_or(0),
                    &mut output,
                )?;
                if output.is_empty() {
                    Ok("No matches found.".to_owned())
                } else {
                    Ok(output)
                }
            }
            "fs.skeleton" => {
                let path = require_str(args, "path")?;
                let full = resolve_path(project?, path);
                let content =
                    std::fs::read_to_string(&full).map_err(|e| ToolError::Io(e.to_string()))?;
                Ok(extract_skeleton(&content))
            }
            "code.fault_localize" => {
                use std::fmt::Write as _;
                use synwire_agent::sbfl::{CoverageRecord, SbflRanker, fuse_sbfl_semantic};

                let coverage_arr = args
                    .get("coverage")
                    .and_then(Value::as_array)
                    .ok_or_else(|| ToolError::MissingParam("coverage".to_owned()))?;

                let mut records = Vec::with_capacity(coverage_arr.len());
                for item in coverage_arr {
                    let file = item
                        .get("file")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let line = item.get("line").and_then(Value::as_u64).unwrap_or(0);
                    let ef = item.get("ef").and_then(Value::as_u64).unwrap_or(0);
                    let ep = item.get("ep").and_then(Value::as_u64).unwrap_or(0);
                    let nf = item.get("nf").and_then(Value::as_u64).unwrap_or(0);
                    let np = item.get("np").and_then(Value::as_u64).unwrap_or(0);
                    records.push(CoverageRecord::new(
                        file,
                        u32::try_from(line).unwrap_or(u32::MAX),
                        u32::try_from(ef).unwrap_or(u32::MAX),
                        u32::try_from(ep).unwrap_or(u32::MAX),
                        u32::try_from(nf).unwrap_or(u32::MAX),
                        u32::try_from(np).unwrap_or(u32::MAX),
                    ));
                }

                let ranker = SbflRanker::new(records);
                let sbfl_ranked = ranker.rank_files();

                let semantic_results: Option<Vec<(String, f32)>> = args
                    .get("semantic_results")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|item| {
                                let file = item.get("file")?.as_str()?.to_owned();
                                #[allow(clippy::cast_possible_truncation)]
                                let score = item.get("score")?.as_f64()? as f32;
                                Some((file, score))
                            })
                            .collect()
                    });

                #[allow(clippy::cast_possible_truncation)]
                let sbfl_weight = args
                    .get("sbfl_weight")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.7) as f32;

                let final_ranked = match semantic_results {
                    Some(ref sem) => fuse_sbfl_semantic(&sbfl_ranked, sem, sbfl_weight),
                    None => sbfl_ranked,
                };

                let mut out = String::from("# Fault Localization Results\n\n");
                let _ = writeln!(out, "| Rank | File | Score |");
                let _ = writeln!(out, "|------|------|-------|");
                for (i, (file, score)) in final_ranked.iter().enumerate() {
                    let _ = writeln!(out, "| {} | {} | {score:.4} |", i + 1, file);
                }
                Ok(out)
            }
            "code.trace_dataflow" => {
                use std::fmt::Write as _;
                use synwire_agent::dataflow::DataflowTracer;

                let path = require_str(args, "file")?;
                let variable = require_str(args, "variable")?;
                let max_hops = args.get("max_hops").and_then(Value::as_u64).unwrap_or(10);
                let full = resolve_path(project?, path);
                let source =
                    std::fs::read_to_string(&full).map_err(|e| ToolError::Io(e.to_string()))?;
                let tracer = DataflowTracer::new(u32::try_from(max_hops).unwrap_or(10));
                let hops = tracer.trace(&source, variable, path);

                if hops.is_empty() {
                    return Ok(format!(
                        "No dataflow origins found for `{variable}` in {}",
                        full.display()
                    ));
                }

                let mut out = format!("# Dataflow trace for `{variable}`\n\n");
                for hop in &hops {
                    let _ = writeln!(
                        out,
                        "- [{}] {}:{} `{}`",
                        hop.origin.kind, hop.origin.file, hop.origin.line, hop.origin.snippet,
                    );
                }
                Ok(out)
            }
            "code.trace_callers" => {
                // code.trace_callers requires LSP goto-definition data that
                // lives daemon-side.  If the daemon is available, proxy the
                // request; otherwise return a helpful message.
                if self.daemon_proxy.is_available() {
                    match self
                        .daemon_proxy
                        .send_request_blocking("call_graph_query", args.clone())
                    {
                        Ok(value) => Ok(serde_json::to_string_pretty(&value)
                            .unwrap_or_else(|_| value.to_string())),
                        Err(e) => Err(ToolError::Other(e.to_string())),
                    }
                } else {
                    Err(ToolError::NotConfigured(
                        "code.trace_callers requires the daemon for LSP-backed call graph \
                         data. Start the daemon with `synwire-daemon`."
                            .to_owned(),
                    ))
                }
            }
            // ---------------------------------------------------------
            // code.* -- Fused tools (LSP + daemon fallback)
            // ---------------------------------------------------------
            "code.search" => {
                // Dispatch to daemon with appropriate method based on mode.
                let mode = args
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("by_meaning");
                let daemon_method = match mode {
                    "by_graph" => "graph_search",
                    "by_community" => "community_search",
                    // by_meaning (default)
                    _ => "semantic_search",
                };
                self.proxy_to_daemon(daemon_method, args)
            }
            "code.search_hybrid" => self.proxy_to_daemon("hybrid_search", args),
            "code.definition" => {
                // If file+line+column are provided and LSP is available, use
                // LSP.
                let has_position = args.get("file").is_some()
                    && args.get("line").is_some()
                    && args.get("column").is_some();
                if has_position && self.options.lsp.is_some() {
                    self.dispatch_lsp_tool("lsp.definition", args)
                } else if let Some(symbol) = args.get("symbol").and_then(|v| v.as_str()) {
                    // Fall back to daemon graph_query with direction=outgoing.
                    let graph_args = serde_json::json!({
                        "symbol": symbol,
                        "direction": "outgoing"
                    });
                    self.proxy_to_daemon("graph_query", &graph_args)
                } else {
                    Err(ToolError::MissingParam(
                        "Provide either file+line+column (LSP) or symbol (graph)".to_owned(),
                    ))
                }
            }
            "code.references" => {
                // If file+line+column are provided and LSP is available, use
                // LSP.
                let has_position = args.get("file").is_some()
                    && args.get("line").is_some()
                    && args.get("column").is_some();
                if has_position && self.options.lsp.is_some() {
                    self.dispatch_lsp_tool("lsp.references", args)
                } else if let Some(symbol) = args.get("symbol").and_then(|v| v.as_str()) {
                    // Try xref_query first, then graph_query with incoming.
                    let xref_args = serde_json::json!({
                        "symbol": symbol,
                        "worktree_id": self.worktree_id_str()
                    });
                    let xref_result = self.proxy_to_daemon("xref_query", &xref_args);
                    if xref_result.is_ok() {
                        return xref_result;
                    }
                    // Fall back to graph_query incoming.
                    let graph_args = serde_json::json!({
                        "symbol": symbol,
                        "direction": "incoming"
                    });
                    self.proxy_to_daemon("graph_query", &graph_args)
                } else {
                    Err(ToolError::MissingParam(
                        "Provide either file+line+column (LSP) or symbol (index)".to_owned(),
                    ))
                }
            }
            "code.symbols" => {
                let file = require_str(args, "file")?;
                // Try LSP document_symbols first if available.
                if self.options.lsp.is_some() {
                    let lsp_result = self.dispatch_lsp_tool("lsp.symbols", args);
                    if lsp_result.is_ok() {
                        return lsp_result;
                    }
                }
                // Fall back to local skeleton extraction.
                let full = resolve_path(project?, file);
                let content =
                    std::fs::read_to_string(&full).map_err(|e| ToolError::Io(e.to_string()))?;
                Ok(extract_skeleton(&content))
            }
            "code.type_info" => {
                // Requires LSP -- no fallback.
                if self.options.lsp.is_some() {
                    self.dispatch_lsp_tool("lsp.hover", args)
                } else {
                    Err(ToolError::NotConfigured(
                        "code.type_info requires --lsp to be configured. \
                         Pass the language server command with --lsp <cmd>."
                            .to_owned(),
                    ))
                }
            }
            // ---------------------------------------------------------
            // index.* -- Remote index tools with local shortcuts
            // ---------------------------------------------------------
            "index.search_docs" => {
                // Proxy to daemon semantic_search with file_type filter.
                let mut proxy_args = args.clone();
                if let Some(obj) = proxy_args.as_object_mut() {
                    let _ = obj.insert(
                        "file_type".to_owned(),
                        serde_json::Value::String("docs".to_owned()),
                    );
                }
                self.proxy_to_daemon("semantic_search", &proxy_args)
            }
            "index.search_docs_hybrid" => {
                // Proxy to daemon hybrid_search with file_type filter.
                let mut proxy_args = args.clone();
                if let Some(obj) = proxy_args.as_object_mut() {
                    let _ = obj.insert(
                        "file_type".to_owned(),
                        serde_json::Value::String("docs".to_owned()),
                    );
                }
                self.proxy_to_daemon("hybrid_search", &proxy_args)
            }
            // ---------------------------------------------------------
            // Remote tools proxied to the daemon via UDS
            // ---------------------------------------------------------
            remote if proxy::is_remote_tool(remote) => {
                if !self.daemon_proxy.is_available() {
                    return Err(ToolError::NotConfigured(
                        "Daemon is not running. Start it with `synwire-daemon` \
                         or ensure the socket exists."
                            .to_owned(),
                    ));
                }
                let result = match remote {
                    "index.build" => {
                        let root = require_str(args, "worktree_root")?;
                        self.daemon_proxy.index_blocking(root)
                    }
                    "code.dependencies" => {
                        let query = require_str(args, "query")?;
                        let wid = require_str(args, "worktree_id")?;
                        self.daemon_proxy.graph_query_blocking(query, wid)
                    }
                    "vcs.clone" => {
                        let url = require_str(args, "url")?;
                        let dest = require_str(args, "dest")?;
                        self.daemon_proxy.clone_repo_blocking(url, dest)
                    }
                    // Remaining remote tools use the generic proxy path.
                    _ => self
                        .daemon_proxy
                        .send_request_blocking(remote, args.clone()),
                };
                match result {
                    Ok(value) => {
                        Ok(serde_json::to_string_pretty(&value)
                            .unwrap_or_else(|_| value.to_string()))
                    }
                    Err(e) => Err(ToolError::Other(e.to_string())),
                }
            }
            // ---------------------------------------------------------
            // lsp.* -- Direct LSP tools (require --lsp)
            // ---------------------------------------------------------
            "lsp.hover" | "lsp.definition" | "lsp.references" | "lsp.symbols" => {
                self.dispatch_lsp_tool(name, args)
            }
            // ---------------------------------------------------------
            // debug.* -- DAP tools (require --dap)
            // ---------------------------------------------------------
            "debug.breakpoint" | "debug.evaluate" => self.dispatch_dap_tool(name, args),
            _ => Err(ToolError::NotFound(name.to_owned())),
        }
    }

    // -- Daemon proxy helper --------------------------------------------------

    /// Proxy a tool call to the daemon via the Unix domain socket.
    ///
    /// Returns the JSON-RPC result as a pretty-printed string, or a
    /// [`ToolError`] if the daemon is unavailable or returns an error.
    fn proxy_to_daemon(&self, method: &str, args: &Value) -> Result<String, ToolError> {
        if !self.daemon_proxy.is_available() {
            return Err(ToolError::NotConfigured(
                "Daemon is not running. Start it with `synwire-daemon` \
                 or ensure the socket exists."
                    .to_owned(),
            ));
        }
        match self
            .daemon_proxy
            .send_request_blocking(method, args.clone())
        {
            Ok(value) => {
                Ok(serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()))
            }
            Err(e) => Err(ToolError::Other(e.to_string())),
        }
    }

    /// Return the current worktree ID as a string, or an empty string if no
    /// project is configured or the worktree ID cannot be resolved.
    fn worktree_id_str(&self) -> String {
        self.options
            .project
            .as_deref()
            .and_then(|p| WorktreeId::for_path(p).ok())
            .map_or_else(String::new, |wid| wid.to_string())
    }

    // -- LSP tool dispatch ---------------------------------------------------

    /// Dispatch an LSP tool call.
    ///
    /// When the `lsp` feature is enabled and `--lsp` is configured, this
    /// lazily starts an `LspClient`, performs the LSP handshake, and delegates
    /// the request. Without the feature flag, a compile-time fallback returns
    /// a helpful message.
    #[cfg(feature = "lsp")]
    fn dispatch_lsp_tool(&self, name: &str, args: &Value) -> Result<String, ToolError> {
        let Some(lsp_cmd) = self.options.lsp.as_deref() else {
            return Ok("LSP tool requires --lsp to be configured. \
                       Pass the language server command with --lsp <cmd>."
                .to_owned());
        };

        // Build a single-threaded tokio runtime for async LSP calls.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ToolError::Other(format!("Failed to create tokio runtime: {e}")))?;

        // Lazily initialise the LSP client on first use.
        let client = self.get_or_init_lsp_client(&rt, lsp_cmd)?;

        match name {
            "lsp.hover" => {
                let file = require_str(args, "file")?;
                let line = require_u32(args, "line")?;
                let col = require_u32(args, "column")?;
                let uri = file_uri(file)?;
                // MCP schema uses 1-based positions; LSP uses 0-based.
                let result = rt
                    .block_on(client.hover(&uri, line.saturating_sub(1), col.saturating_sub(1)))
                    .map_err(|e| ToolError::Other(e.to_string()))?;
                serde_json::to_string_pretty(&result).map_err(|e| ToolError::Other(e.to_string()))
            }
            "lsp.definition" => {
                let file = require_str(args, "file")?;
                let line = require_u32(args, "line")?;
                let col = require_u32(args, "column")?;
                let uri = file_uri(file)?;
                let result = rt
                    .block_on(client.goto_definition(
                        &uri,
                        line.saturating_sub(1),
                        col.saturating_sub(1),
                    ))
                    .map_err(|e| ToolError::Other(e.to_string()))?;
                serde_json::to_string_pretty(&result).map_err(|e| ToolError::Other(e.to_string()))
            }
            "lsp.references" => {
                let file = require_str(args, "file")?;
                let line = require_u32(args, "line")?;
                let col = require_u32(args, "column")?;
                let uri = file_uri(file)?;
                let result = rt
                    .block_on(client.find_references(
                        &uri,
                        line.saturating_sub(1),
                        col.saturating_sub(1),
                    ))
                    .map_err(|e| ToolError::Other(e.to_string()))?;
                serde_json::to_string_pretty(&result).map_err(|e| ToolError::Other(e.to_string()))
            }
            "lsp.symbols" => {
                let file = require_str(args, "file")?;
                let uri = file_uri(file)?;
                let result = rt
                    .block_on(client.document_symbols(&uri))
                    .map_err(|e| ToolError::Other(e.to_string()))?;
                serde_json::to_string_pretty(&result).map_err(|e| ToolError::Other(e.to_string()))
            }
            _ => Err(ToolError::NotFound(name.to_owned())),
        }
    }

    /// Fallback when the `lsp` feature is not enabled.
    #[cfg(not(feature = "lsp"))]
    #[allow(clippy::unnecessary_wraps)]
    fn dispatch_lsp_tool(&self, _name: &str, _args: &Value) -> Result<String, ToolError> {
        if self.options.lsp.is_none() {
            Ok("LSP tool requires --lsp to be configured.                 Pass the language server command with --lsp <cmd>."
                .to_owned())
        } else {
            Ok("LSP tools require the `lsp` feature to be enabled at compile time.                 Rebuild with: cargo install synwire-mcp-server --features lsp"
                .to_owned())
        }
    }

    /// Get or lazily initialise the shared LSP client.
    #[cfg(feature = "lsp")]
    fn get_or_init_lsp_client(
        &self,
        rt: &tokio::runtime::Runtime,
        lsp_cmd: &str,
    ) -> Result<std::sync::Arc<synwire_lsp::client::LspClient>, ToolError> {
        // Fast path: already initialised.
        {
            let guard = self
                .lsp_client
                .read()
                .map_err(|e| ToolError::Other(format!("LSP lock poisoned: {e}")))?;
            if let Some(ref c) = *guard {
                return Ok(std::sync::Arc::clone(c));
            }
        }

        // Slow path: initialise under write lock.
        let mut guard = self
            .lsp_client
            .write()
            .map_err(|e| ToolError::Other(format!("LSP lock poisoned: {e}")))?;

        // Double-check after acquiring write lock.
        if let Some(ref c) = *guard {
            return Ok(std::sync::Arc::clone(c));
        }

        // Parse command string: first token is the binary, rest are args.
        let parts: Vec<&str> = lsp_cmd.split_whitespace().collect();
        let (cmd, cmd_args) = parts
            .split_first()
            .ok_or_else(|| ToolError::NotConfigured("Empty --lsp command".to_owned()))?;

        let mut config = synwire_lsp::config::LspServerConfig::new((*cmd).to_owned());
        config.args = cmd_args.iter().map(|s| (*s).to_owned()).collect();
        config.root_uri = self.options.project.as_ref().and_then(|p| {
            lsp_types::Url::from_file_path(p)
                .ok()
                .map(|u| u.to_string())
        });

        let client = synwire_lsp::client::LspClient::start(&config)
            .map_err(|e| ToolError::Other(format!("Failed to start LSP server: {e}")))?;

        // Perform the initialize handshake.
        if let Some(ref project) = self.options.project {
            if let Ok(root_uri) = lsp_types::Url::from_file_path(project) {
                rt.block_on(client.initialize_with_root(&root_uri))
                    .map_err(|e| ToolError::Other(format!("LSP initialize failed: {e}")))?;
            } else {
                rt.block_on(client.initialize())
                    .map_err(|e| ToolError::Other(format!("LSP initialize failed: {e}")))?;
            }
        } else {
            rt.block_on(client.initialize())
                .map_err(|e| ToolError::Other(format!("LSP initialize failed: {e}")))?;
        }

        info!("LSP client initialised for command: {lsp_cmd}");
        let arc = std::sync::Arc::new(client);
        *guard = Some(std::sync::Arc::clone(&arc));
        drop(guard);
        Ok(arc)
    }

    // -- DAP tool dispatch ---------------------------------------------------

    /// Dispatch a DAP tool call.
    ///
    /// When the `dap` feature is enabled and `--dap` is configured, this
    /// lazily starts a `DapClient`, performs the DAP handshake, and delegates
    /// the request.
    #[cfg(feature = "dap")]
    fn dispatch_dap_tool(&self, name: &str, args: &Value) -> Result<String, ToolError> {
        let Some(dap_cmd) = self.options.dap.as_deref() else {
            return Ok("DAP tool requires --dap to be configured. \
                       Pass the debug adapter command with --dap <cmd>."
                .to_owned());
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ToolError::Other(format!("Failed to create tokio runtime: {e}")))?;

        let client = self.get_or_init_dap_client(&rt, dap_cmd)?;

        match name {
            "debug.breakpoint" => {
                let file = require_str(args, "file")?;
                let line = args
                    .get("line")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| ToolError::MissingParam("line".to_owned()))?;
                let result = rt
                    .block_on(client.set_breakpoints(file, &[line]))
                    .map_err(|e| ToolError::Other(e.to_string()))?;
                serde_json::to_string_pretty(&serde_json::json!({ "breakpoints": result }))
                    .map_err(|e| ToolError::Other(e.to_string()))
            }
            "debug.evaluate" => {
                let expression = require_str(args, "expression")?;
                let frame_id = args.get("frame_id").and_then(Value::as_i64);
                let result = rt
                    .block_on(client.evaluate(expression, frame_id))
                    .map_err(|e| ToolError::Other(e.to_string()))?;
                serde_json::to_string_pretty(&result).map_err(|e| ToolError::Other(e.to_string()))
            }
            _ => Err(ToolError::NotFound(name.to_owned())),
        }
    }

    /// Fallback when the `dap` feature is not enabled.
    #[cfg(not(feature = "dap"))]
    #[allow(clippy::unnecessary_wraps)]
    fn dispatch_dap_tool(&self, _name: &str, _args: &Value) -> Result<String, ToolError> {
        if self.options.dap.is_none() {
            Ok("DAP tool requires --dap to be configured.                 Pass the debug adapter command with --dap <cmd>."
                .to_owned())
        } else {
            Ok("DAP tools require the `dap` feature to be enabled at compile time.                 Rebuild with: cargo install synwire-mcp-server --features dap"
                .to_owned())
        }
    }

    /// Get or lazily initialise the shared DAP client.
    #[cfg(feature = "dap")]
    fn get_or_init_dap_client(
        &self,
        rt: &tokio::runtime::Runtime,
        dap_cmd: &str,
    ) -> Result<std::sync::Arc<synwire_dap::DapClient>, ToolError> {
        // Fast path: already initialised.
        {
            let guard = self
                .dap_client
                .read()
                .map_err(|e| ToolError::Other(format!("DAP lock poisoned: {e}")))?;
            if let Some(ref c) = *guard {
                return Ok(std::sync::Arc::clone(c));
            }
        }

        // Slow path: initialise under write lock.
        let mut guard = self
            .dap_client
            .write()
            .map_err(|e| ToolError::Other(format!("DAP lock poisoned: {e}")))?;

        // Double-check after acquiring write lock.
        if let Some(ref c) = *guard {
            return Ok(std::sync::Arc::clone(c));
        }

        let parts: Vec<&str> = dap_cmd.split_whitespace().collect();
        let (cmd, cmd_args) = parts
            .split_first()
            .ok_or_else(|| ToolError::NotConfigured("Empty --dap command".to_owned()))?;

        let mut config = synwire_dap::DapAdapterConfig::new((*cmd).to_owned());
        config.args = cmd_args.iter().map(|s| (*s).to_owned()).collect();

        // No-op event handler for MCP server context. DAP events are logged
        // via tracing but not forwarded to any UI.
        let event_handler: std::sync::Arc<dyn Fn(serde_json::Value) + Send + Sync> =
            std::sync::Arc::new(|event| {
                debug!(event = %event, "DAP event received");
            });

        let client = synwire_dap::DapClient::start(&config, event_handler)
            .map_err(|e| ToolError::Other(format!("Failed to start DAP adapter: {e}")))?;

        rt.block_on(client.initialize())
            .map_err(|e| ToolError::Other(format!("DAP initialize failed: {e}")))?;

        info!("DAP client initialised for command: {dap_cmd}");
        let arc = std::sync::Arc::new(client);
        *guard = Some(std::sync::Arc::clone(&arc));
        drop(guard);
        Ok(arc)
    }
}

// ---------------------------------------------------------------------------
// Tool dispatch helpers
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Not configured: {0}")]
    NotConfigured(String),
    #[error("Missing parameter '{0}'")]
    MissingParam(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::MissingParam(key.to_owned()))
}

/// Extract a required `u32` parameter from a JSON value.
#[allow(dead_code)]
fn require_u32(args: &Value, key: &str) -> Result<u32, ToolError> {
    args.get(key)
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| ToolError::MissingParam(key.to_owned()))
}

/// Convert a file path string to an `lsp_types::Url`.
#[cfg(feature = "lsp")]
fn file_uri(path: &str) -> Result<lsp_types::Url, ToolError> {
    lsp_types::Url::from_file_path(path)
        .map_err(|()| ToolError::Other(format!("Invalid file path: {path}")))
}

fn resolve_path(base: &std::path::Path, path: &str) -> PathBuf {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

fn dir_tree(path: &std::path::Path, max_depth: usize, depth: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let indent = "  ".repeat(depth);
    let name = path.file_name().map_or(".", |n| n.to_str().unwrap_or("."));
    let _ = writeln!(out, "{indent}{name}/");
    if depth >= max_depth {
        return out;
    }
    if let Ok(entries) = std::fs::read_dir(path) {
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let ft = entry.file_type();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }
            match ft {
                Ok(ft) if ft.is_dir() => {
                    out.push_str(&dir_tree(&entry.path(), max_depth, depth + 1));
                }
                Ok(_) => {
                    let _ = writeln!(out, "{}{name_str}", "  ".repeat(depth + 1));
                }
                Err(_) => {}
            }
        }
    }
    out
}

fn glob_files(base: &std::path::Path, pattern: &str) -> Result<Vec<String>, ToolError> {
    use globset::{Glob, GlobSetBuilder};
    let glob = Glob::new(pattern).map_err(|e| ToolError::Other(e.to_string()))?;
    let mut builder = GlobSetBuilder::new();
    let _ = builder.add(glob);
    let set = builder
        .build()
        .map_err(|e| ToolError::Other(e.to_string()))?;

    let mut results = Vec::new();
    walk_for_glob(base, base, &set, &mut results);
    Ok(results)
}

fn walk_for_glob(
    root: &std::path::Path,
    dir: &std::path::Path,
    set: &globset::GlobSet,
    out: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(rel) = path.strip_prefix(root)
            && set.is_match(rel)
        {
            out.push(rel.to_string_lossy().into_owned());
        }
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            walk_for_glob(root, &path, set, out);
        }
    }
}

/// Recursively search files under `dir` for lines matching `re`, appending
/// results with optional context lines to `out`.
fn grep_recursive(
    dir: &std::path::Path,
    re: &regex::Regex,
    context: usize,
    out: &mut String,
) -> Result<(), ToolError> {
    use std::fmt::Write as _;

    let entries = std::fs::read_dir(dir).map_err(|e| ToolError::Io(e.to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            grep_recursive(&path, re, context, out)?;
            continue;
        }
        // Skip binary files by checking if the file reads as valid UTF-8.
        let Ok(file_text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let lines: Vec<&str> = file_text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if re.is_match(line) {
                let start = i.saturating_sub(context);
                let end = (i + context + 1).min(lines.len());
                for (j, matched_line) in lines.iter().enumerate().take(end).skip(start) {
                    let marker = if j == i { ">" } else { " " };
                    let _ = writeln!(
                        out,
                        "{}:{}:{} {}",
                        path.display(),
                        j + 1,
                        marker,
                        matched_line
                    );
                }
                if context > 0 {
                    let _ = writeln!(out, "--");
                }
            }
        }
    }
    Ok(())
}

/// Extract a skeleton (structural lines) from source code.
///
/// Returns lines containing `fn `, `struct `, `enum `, `trait `, `impl `,
/// `pub `, or `mod ` keywords, giving a token-efficient overview of the file.
fn extract_skeleton(source: &str) -> String {
    use std::fmt::Write as _;
    let keywords = ["fn ", "struct ", "enum ", "trait ", "impl ", "pub ", "mod "];
    let mut out = String::new();
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if keywords.iter().any(|kw| trimmed.contains(kw)) {
            let _ = writeln!(out, "{:>4}: {}", i + 1, line);
        }
    }
    if out.is_empty() {
        "No structural lines found.".to_owned()
    } else {
        out
    }
}

/// Synchronously scan a skills directory for agent skill manifests and return
/// each skill as an [`McpTool`].
///
/// Mirrors the logic of `SkillLoader::scan` but uses blocking I/O so it can
/// be called from a non-async context during server initialisation.
fn scan_skills_sync(
    skills_dir: &std::path::Path,
) -> Result<Vec<McpTool>, Box<dyn std::error::Error + Send + Sync>> {
    use synwire_agent_skills::loader::{SkillEntry, SkillLoader};
    use synwire_agent_skills::manifest::parse_skill_md;

    let loader = SkillLoader::new();
    let mut tools = Vec::new();

    let read_dir = std::fs::read_dir(skills_dir)?;
    for entry in read_dir.flatten() {
        let child_path = entry.path();
        if !child_path.is_dir() {
            continue;
        }
        let skill_file = child_path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&skill_file) {
            Ok(c) => c,
            Err(e) => {
                warn!(path = %skill_file.display(), error = %e, "Failed to read SKILL.md");
                continue;
            }
        };

        let manifest = match parse_skill_md(&content) {
            Ok(m) => m,
            Err(e) => {
                warn!(path = %skill_file.display(), error = %e, "Failed to parse SKILL.md");
                continue;
            }
        };

        // Use SkillLoader::validate to enforce structural invariants.
        let entry_for_validate = SkillEntry {
            manifest: manifest.clone(),
            body: String::new(),
            skill_dir: child_path,
        };
        if let Err(e) = loader.validate(&entry_for_validate) {
            warn!(
                name = %manifest.name,
                error = %e,
                "Skipping invalid skill"
            );
            continue;
        }

        let tool = McpTool {
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "args": { "type": "object", "description": "Skill arguments" }
                }
            }),
        };
        tools.push(tool);
    }

    Ok(tools)
}

# synwire-mcp-server: MCP Server Binary

`synwire-mcp-server` is a standalone binary that exposes Synwire's tools over the [Model Context Protocol](https://modelcontextprotocol.io) via a stdio JSON-RPC 2.0 transport. It is the primary integration point for using Synwire from MCP-compatible hosts such as Claude Desktop, Cursor, Windsurf, or any other MCP client.

## Architecture

The server follows a thin-proxy design:

```text
MCP Client (Claude Desktop, etc.)
    |  stdio (JSON-RPC 2.0)
    v
synwire-mcp-server
    |
    +-- Built-in tools (file ops, search, etc.)
    +-- Agent skills (from $DATA/<product>/skills/)
    +-- LSP tools (optional, feature-gated)
    +-- DAP tools (optional, feature-gated)
    +-- DaemonProxy (forwards to synwire-daemon)
    +-- ToolSearchIndex (progressive discovery)
```

All diagnostic output goes to stderr. Stdout is reserved exclusively for MCP protocol messages.

### Protocol

The server implements three MCP methods:

| Method | Description |
|---|---|
| `initialize` | Returns server capabilities, name, and version |
| `tools/list` | Returns tool definitions (filtered by `ToolSearchIndex` when progressive discovery is active) |
| `tools/call` | Invokes a tool and returns its result |

### `McpServer`

The central runtime type. Holds:

- **`ServerOptions`** --- resolved configuration from CLI flags and config file
- **`StorageLayout`** --- product-scoped paths for data, cache, and logs
- **Tool registry** --- `HashMap<String, McpTool>` of all registered tools
- **`ToolSearchIndex`** --- progressive tool discovery index that reduces token usage by exposing only relevant tools per query
- **`DaemonProxy`** --- forwards tool calls to the `synwire-daemon` singleton when it is running
- **`McpSampling`** --- placeholder for MCP sampling support (tool-internal LLM access via `sampling/createMessage`)

### Tool registration

At startup, `McpServer::new` registers tools from three sources:

1. **Built-in tools** --- `builtin_tools()` returns the core set of file, search, and management tools.
2. **Agent skills** --- the global skills directory (`$DATA/<product>/skills/`) is scanned via `synwire-agent-skills`. Each discovered skill becomes an MCP tool.
3. **LSP/DAP tools** --- when the `lsp` or `dap` features are enabled and the corresponding CLI flag is set, language server and debug adapter tools are registered.

All tools are indexed in the `ToolSearchIndex` for progressive discovery.

### LSP integration

When `--lsp <command>` is passed (e.g. `--lsp rust-analyzer`), the server registers LSP tools: `lsp.hover`, `lsp.definition`, `lsp.references`, `lsp.symbols`, and others. The LSP client is initialised lazily on first tool call. Requires the `lsp` feature flag.

### DAP integration

When `--dap <command>` is passed (e.g. `--dap lldb-dap`), the server registers DAP tools: `debug.breakpoint`, `debug.evaluate`, and others. The DAP client is also initialised lazily. Requires the `dap` feature flag.

### Multi-instance safety

Multiple MCP server instances can safely share the same data directories. `StorageLayout` uses SQLite WAL mode for all databases, and LanceDB handles concurrent access natively. No external file locks are needed.

## `ServerOptions`

| Field | Description |
|---|---|
| `project` | Project root directory (enables project-scoped indexing) |
| `product_name` | Product name for storage scoping (e.g. `"synwire"`) |
| `embedding_model` | Model identifier for tool search and semantic indexing |
| `lsp` | LSP server command (optional) |
| `dap` | DAP server command (optional) |

## Feature flags

| Flag | Enables |
|---|---|
| `lsp` | LSP tool dispatch via `synwire-lsp` |
| `dap` | DAP tool dispatch via `synwire-dap` |

## Dependencies

| Crate | Role |
|---|---|
| `synwire-core` | Tool traits, `ToolSearchIndex`, `SamplingProvider` |
| `synwire-agent` | Agent runtime for tool execution |
| `synwire-agent-skills` | Skill discovery and registration |
| `synwire-storage` | `StorageLayout`, `WorktreeId` |
| `synwire-index` | Semantic indexing pipeline |
| `synwire-mcp-adapters` | MCP protocol utilities |
| `synwire-lsp` | LSP client (optional) |
| `synwire-dap` | DAP client (optional) |
| `clap` | CLI argument parsing |
| `tracing` / `tracing-subscriber` / `tracing-appender` | Structured logging to files |

## See also

- [Getting Started with the MCP Server](../tutorials/11-mcp-server.md) --- tutorial
- [Advanced MCP Server Setup](../tutorials/17-advanced-mcp-setup.md) --- advanced configuration
- [Building a Custom MCP Server](../tutorials/19-build-mcp-server.md) --- extending the server
- [synwire-mcp-adapters](./synwire-mcp-adapters.md) --- the client-side MCP infrastructure
- [synwire-agent-skills](./synwire-agent-skills.md) --- how skills become tools

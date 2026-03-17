# synwire-mcp-server

Standalone MCP (Model Context Protocol) server that exposes Synwire tools via stdio JSON-RPC. Integrates with Claude Code, GitHub Copilot, Cursor, and any other MCP host.

## Overview

The MCP server composes tools from multiple Synwire crates into a single `CompositeToolProvider`, then serves them over the MCP stdio transport. Each tool is namespaced (e.g., `fs.read`, `code.search`) so the LLM can understand organisation at a glance.

Internally the server connects to the `synwire-daemon`, a singleton background process that owns the embedding model, file watchers, indexing pipelines, and persistent state. Multiple MCP server instances share a single daemon. VFS file operations that do not require the index are handled directly by the server.

## Installation

```sh
cargo install synwire-mcp-server
```

## Claude Code setup

Add to `.claude/mcp.json` in your project or home directory:

```json
{
  "synwire": {
    "command": "synwire-mcp-server",
    "args": ["--project", "."]
  }
}
```

With LSP and DAP support:

```json
{
  "synwire": {
    "command": "synwire-mcp-server",
    "args": [
      "--project", ".",
      "--lsp", "rust-analyzer",
      "--dap", "codelldb"
    ]
  }
}
```

## Tool Reference

Tools are grouped by namespace. Use `meta.tool_search` to discover relevant tools by natural language query, or `meta.tool_list` to browse all namespaces.

### `fs.*` -- File Operations (6 tools)

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `fs.read` | Read the full contents of a file | `path` |
| `fs.write` | Write content to a file (creates if absent) | `path`, `content` |
| `fs.edit` | Replace exact text in a file | `path`, `old_string`, `new_string` |
| `fs.grep` | Search file contents by regex with context lines | `pattern`, `path`, `context_lines` |
| `fs.glob` | Find files by name pattern | `pattern` |
| `fs.tree` | Show directory structure as an indented tree | `path`, `max_depth` |

### `code.*` -- Code Analysis (11 tools)

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `code.search` | Semantic, graph, or community-based code search | `query`, `mode`, `top_k` |
| `code.definition` | Jump to symbol definition (LSP + graph fallback) | `file`, `line`, `column`, `symbol` |
| `code.references` | Find all references to a symbol | `file`, `line`, `column`, `symbol` |
| `code.skeleton` | Show function/method signatures without bodies | `path` |
| `code.trace_callers` | Query callers of a symbol from the code graph | `symbol`, `depth` |
| `code.trace_callees` | Query callees of a symbol from the code graph | `symbol`, `depth` |
| `code.trace_dataflow` | Trace variable assignments backward through a file | `file`, `variable`, `max_hops` |
| `code.fault_localize` | Rank files by SBFL/Ochiai fault likelihood | `coverage`, `semantic_results`, `sbfl_weight` |
| `code.community_search` | Find code communities matching a query | `query`, `top_k` |
| `code.community_members` | List members of a specific community | `community_id` |
| `code.graph_query` | Raw graph query for imports, definitions, edges | `symbol`, `direction`, `edge_type` |

### `lsp.*` -- Language Intelligence (4 tools, requires `--lsp`)

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `lsp.hover` | Get type information and documentation at a position | `file`, `line`, `column` |
| `lsp.goto_definition` | Jump to where a symbol is defined | `file`, `line`, `column` |
| `lsp.references` | Find all references to a symbol across the project | `file`, `line`, `column` |
| `lsp.document_symbols` | List all symbols in a file | `file` |

### `index.*` -- Indexing and Search (4 tools)

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `index.run` | Index the project for semantic search (incremental) | `path` |
| `index.status` | Check indexing progress and freshness | -- |
| `index.search` | Natural language search over indexed code | `query`, `top_k`, `file_filter` |
| `index.hybrid_search` | Combined BM25 + vector search with alpha weighting | `query`, `alpha`, `top_k` |

### `vcs.*` -- Version Control (1 tool)

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `vcs.clone_repo` | Clone a git repository and mount it as a VFS provider | `url`, `branch` |

### `debug.*` -- Debugging (14 tools, requires `--dap`)

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `debug.launch` | Launch a debug session for a binary | `program`, `args`, `cwd` |
| `debug.attach` | Attach to a running process | `pid` |
| `debug.set_breakpoints` | Set breakpoints at file and line | `file`, `breakpoints` |
| `debug.remove_breakpoints` | Remove breakpoints from a file | `file`, `lines` |
| `debug.continue` | Continue execution until next breakpoint | -- |
| `debug.step_over` | Step over current line | -- |
| `debug.step_into` | Step into function call | -- |
| `debug.step_out` | Step out of current function | -- |
| `debug.pause` | Pause execution | -- |
| `debug.variables` | List variables in the current scope | `scope` |
| `debug.evaluate` | Evaluate an expression in the current context | `expression`, `frame_id` |
| `debug.stack_trace` | Get the current call stack | -- |
| `debug.disconnect` | End the debug session | `terminate` |
| `debug.collect_coverage` | Collect code coverage from the debug session | -- |

### `meta.*` -- Tool Discovery (2 tools)

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `meta.tool_search` | Discover tools by natural language query or namespace | `query`, `namespace`, `top_k` |
| `meta.tool_list` | List all tools grouped by namespace | -- |

## Configuration

### CLI Flags

| Flag | Short | Default | Env var | Description |
|------|-------|---------|---------|-------------|
| `--project <PATH>` | `-p` | -- | `SYNWIRE_PROJECT` | Root directory of the project to index and serve |
| `--product-name <NAME>` | -- | `synwire` | `SYNWIRE_PRODUCT` | Product name for storage path scoping |
| `--lsp <COMMAND>` | -- | -- | `SYNWIRE_LSP` | Language server command (e.g. `rust-analyzer`) |
| `--dap <COMMAND>` | -- | -- | `SYNWIRE_DAP` | Debug adapter command (e.g. `codelldb`) |
| `--embedding-model <MODEL>` | -- | `bge-small-en-v1.5` | `SYNWIRE_EMBEDDING_MODEL` | Embedding model identifier |
| `--enable-meta-tools` | -- | `true` | -- | Register `meta.tool_search` and `meta.tool_list` (disable with `--no-meta-tools`) |
| `--log-level <LEVEL>` | -- | `info` | `RUST_LOG` | Log verbosity: `error`, `warn`, `info`, `debug`, `trace` |
| `--config <PATH>` | `-c` | -- | -- | Path to a TOML or JSON config file |

### Config file

CLI flags take precedence. Format is inferred from file extension.

```toml
project = "/path/to/project"
product_name = "myapp"
lsp = "rust-analyzer"
dap = "codelldb"
embedding_model = "bge-small-en-v1.5"
log_level = "info"
```

### Examples

```sh
# Index and serve current directory
synwire-mcp-server --project .

# With Rust language server and debug adapter
synwire-mcp-server --project . --lsp rust-analyzer --dap codelldb

# Custom product name (isolates storage)
synwire-mcp-server --project /path/to/repo --product-name myapp

# Debug log level
synwire-mcp-server --project . --log-level debug
```

## Architecture

```text
Host (Claude Code / Cursor / IDE)
  |  stdio JSON-RPC
  v
synwire-mcp-server  (thin proxy, handles fs.* directly)
  |  Unix domain socket
  v
synwire-daemon  (singleton, owns indices + LSP + DAP + graphs)
```

The `CompositeToolProvider` inside the server merges tools from:

- `vfs_tools()` -- `fs.*` namespace (handled directly)
- `code_tool_provider()` -- `code.*` namespace (proxied to daemon for graph/search)
- `index_tool_provider()` -- `index.*` namespace (proxied to daemon)
- `lsp_tool_provider()` -- `lsp.*` namespace (managed by daemon)
- `debug_tool_provider()` -- `debug.*` namespace (managed by daemon)
- `meta_tool_provider()` -- `meta.*` namespace (handled directly)
- Agent skills -- discovered from `.<product>/skills/` and `$DATA/<product>/skills/`

The daemon starts automatically on first connection and exits 5 minutes after the last client disconnects.

## Logging

All log output goes to **stderr** -- stdout is reserved for the MCP protocol.

Logs are also written to rotating daily files under `StorageLayout::logs_dir()`:

- Linux: `~/.local/share/<product>/logs/synwire-mcp-server.log.YYYY-MM-DD`
- macOS: `~/Library/Application Support/<product>/logs/`

## Multi-instance safety

Multiple `synwire-mcp-server` instances pointing at the same project are safe:

- SQLite databases use WAL mode -- concurrent readers and writers are allowed
- LanceDB and tantivy have native concurrent access
- One instance indexing while another queries is supported

## Storage layout

Data is stored under `StorageLayout` paths scoped to `--product-name`:

- Indices: `$CACHE/<product>/indices/<worktree_key>/`
- Graphs: `$CACHE/<product>/graphs/<worktree_key>/`
- Sessions: `$DATA/<product>/sessions/<session_id>.db`
- Skills: `$DATA/<product>/skills/`
- Logs: `$DATA/<product>/logs/`

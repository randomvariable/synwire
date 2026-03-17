# Advanced MCP Server Setup

**Time**: ~1 hour
**Prerequisites**: Rust 1.85+, `synwire-mcp-server` installed, completion of [Tutorial 11: Getting Started with the MCP Server](./11-mcp-server.md)

This tutorial covers advanced `synwire-mcp-server` configuration: enabling LSP and DAP tools, setting up the daemon for multi-project indexing, and using namespace-based tool discovery to reduce token usage.

---

## MCP server architecture

The `synwire-mcp-server` binary runs as a stdio-based MCP server. It communicates with the host (Claude Code, an IDE, or your own agent) over stdin/stdout using the MCP protocol.

Internally it connects to the `synwire-daemon`, a singleton background process that owns the embedding model, file watchers, indexing pipelines, and all persistent state. Multiple MCP server instances share a single daemon.

```text
Host (Claude Code)
  |  stdio
  v
synwire-mcp-server  (thin proxy)
  |  Unix domain socket
  v
synwire-daemon  (singleton, owns indices + LSP + DAP)
```

---

## Step 1: Basic configuration

The minimal configuration indexes a single project:

```json
{
  "mcpServers": {
    "synwire": {
      "command": "synwire-mcp-server",
      "args": ["--project", "."]
    }
  }
}
```

This registers all VFS tools (`fs.read`, `fs.write`, `fs.edit`, `fs.grep`, `fs.glob`, `fs.tree`, `code.skeleton`), indexing tools (`index.run`, `index.status`, `index.search`), and meta tools (`meta.tool_search`, `meta.tool_list`).

---

## Step 2: LSP tools (auto-detected by default)

When `--project` is set, the MCP server **automatically detects** which language
servers to use. It scans the project directory for file extensions, matches them
against the built-in `LanguageServerRegistry` (22+ servers), and checks if the
binary is on your `PATH`.

With the basic config from Step 1 and `rust-analyzer` installed, a Rust project
gets LSP tools with zero extra flags:

```json
{
  "mcpServers": {
    "synwire": {
      "command": "synwire-mcp-server",
      "args": ["--project", "."]
    }
  }
}
```

Auto-detection enables four additional tools:

| Tool | Description |
|------|-------------|
| `lsp.hover` | Get type information and documentation for a symbol at a position |
| `lsp.goto_definition` | Jump to where a symbol is defined |
| `lsp.references` | Find all references to a symbol |
| `lsp.document_symbols` | List all symbols in a file |

The MCP server launches the language server as a child process, manages document synchronisation, and translates MCP tool calls into LSP requests.

### Overriding auto-detection

Use `--lsp` to force a specific server (e.g., when multiple servers handle the
same language):

```sh
# Force pyright instead of auto-detected pylsp
synwire-mcp-server --project . --lsp pyright
```

### Disabling LSP entirely

Use `--no-lsp` to disable auto-detection and all LSP tools:

```sh
synwire-mcp-server --project . --no-lsp
```

### Polyglot repos

Auto-detection supports polyglot repositories. If your project contains both
`.rs` and `.ts` files and both `rust-analyzer` and `typescript-language-server`
are installed, both are detected. The server logs which servers were found:

```text
INFO Auto-detected language servers servers=["rust-analyzer", "typescript-language-server"]
```

Currently the primary server (most common extension) is used for tool dispatch.
Future versions will support concurrent multi-server dispatch.

### Supported language servers

The built-in `LanguageServerRegistry` includes definitions for 22+ language servers. Common examples:

| Language | Command |
|----------|---------|
| Rust | `rust-analyzer` |
| TypeScript | `typescript-language-server` |
| Python | `pyright` or `pylsp` |
| Go | `gopls` |
| C/C++ | `clangd` |

```sh
# Rust project (auto-detected, or explicit)
synwire-mcp-server --project . --lsp rust-analyzer

# Python project
synwire-mcp-server --project . --lsp pyright

# Go project
synwire-mcp-server --project . --lsp gopls
```

---

## Step 3: DAP tools (auto-detected by default)

Like LSP, debug adapters are **auto-detected** when `--project` is set. The
server maps detected file extensions to language identifiers and checks if a
matching adapter binary is on `PATH`.

For a Rust project with `codelldb` installed, debugging tools appear
automatically:

```json
{
  "mcpServers": {
    "synwire": {
      "command": "synwire-mcp-server",
      "args": ["--project", "."]
    }
  }
}
```

Use `--dap` to override, or `--no-dap` to disable:

```sh
# Override: force a specific adapter
synwire-mcp-server --project . --dap codelldb

# Disable all DAP tools
synwire-mcp-server --project . --no-dap
```

This enables debugging tools:

| Tool | Description |
|------|-------------|
| `debug.set_breakpoints` | Set breakpoints at a file and line |
| `debug.evaluate` | Evaluate an expression in the current debug context |

The DAP plugin also exposes session management tools (`debug.launch`, `debug.attach`, `debug.continue`, `debug.step_over`, `debug.variables`, etc.) through the agent plugin system.

---

## Step 4: Tool discovery with `tool_search`

With LSP, DAP, VFS, indexing, and analysis tools all registered, the full tool set exceeds 25 tools. Listing all of them in every prompt wastes tokens. The `tool_search` and `tool_list` meta-tools solve this.

### How it works

At startup, the server registers all tools with a `ToolSearchIndex`. Tools are grouped into namespaces:

| Namespace | Tools |
|-----------|-------|
| `fs` | `fs.read`, `fs.write`, `fs.edit`, `fs.grep`, `fs.glob`, `fs.tree` |
| `code` | `code.search`, `code.definition`, `code.references`, `code.skeleton`, `code.trace_callers`, `code.trace_callees`, `code.trace_dataflow`, `code.fault_localize`, `code.community_search`, `code.community_members`, `code.graph_query` |
| `index` | `index.run`, `index.status`, `index.search`, `index.hybrid_search` |
| `lsp` | `lsp.hover`, `lsp.goto_definition`, `lsp.references`, `lsp.document_symbols` |
| `debug` | `debug.launch`, `debug.attach`, `debug.set_breakpoints`, `debug.evaluate`, ... (14 total) |
| `vcs` | `vcs.clone_repo` |
| `meta` | `meta.tool_search`, `meta.tool_list` |

### Browse a namespace

```json
{"tool": "meta.tool_search", "arguments": {"namespace": "lsp"}}
```

Returns all LSP tools with name and description.

### Search by intent

```json
{"tool": "meta.tool_search", "arguments": {"query": "find where a function is defined"}}
```

Returns the most relevant tools ranked by keyword + semantic similarity.

### Progressive discovery

The index applies adaptive scoring: tools already returned in previous searches get a 0.8x penalty, surfacing unseen tools in subsequent queries. This naturally guides the agent toward tools it has not yet tried.

---

## Step 5: Agent skills integration

Agent skills are reusable tool bundles following the [agentskills.io](https://agentskills.io) specification. They live in two locations:

- **Global**: `$XDG_DATA_HOME/synwire/skills/` (shared across projects)
- **Project-local**: `.synwire/skills/` (project-specific)

Each skill is a directory containing:

```text
my-skill/
  SKILL.md         # Name, description, parameters, runtime
  scripts/         # Lua, Rhai, or WASM implementation
  references/      # Documentation the skill can read
  assets/          # Static files
```

The MCP server discovers skills at startup and registers them with `tool_search`. The agent sees only name and description until it activates a skill, at which point the full body is loaded.

### Creating a project-local skill

```sh
mkdir -p .synwire/skills/lint-fix
```

Create `.synwire/skills/lint-fix/SKILL.md`:

```markdown
---
name: lint-fix
description: Run the project linter and auto-fix warnings
runtime: tool-sequence
parameters:
  - name: path
    type: string
    description: File or directory to lint
---

1. Run `cargo clippy --fix --allow-dirty` on the target path
2. Read the output and report any remaining warnings
```

The `tool-sequence` runtime means the skill is a sequence of existing tool calls -- no scripting needed. The MCP server expands the steps into tool calls at invocation time.

---

## Step 6: Daemon configuration

The daemon starts automatically when the first MCP server connects and stops 5 minutes after the last client disconnects. You can configure it via environment variables or a config file:

```sh
# Environment variables
export SYNWIRE_PRODUCT=myapp          # Isolates storage from other instances
export SYNWIRE_EMBEDDING_MODEL=bge-small-en-v1.5
export RUST_LOG=synwire=debug

# Config file (~/.config/synwire/config.toml)
[daemon]
embedding_model = "bge-small-en-v1.5"
log_level = "info"
grace_period_secs = 300
```

### Multi-project indexing

The daemon identifies projects by `RepoId` (git first-commit hash) and `WorktreeId` (repo + worktree path). Multiple MCP servers pointing at different worktrees of the same repo share a single `RepoId` but maintain separate indices.

```sh
# Terminal 1: main branch
cd ~/projects/myapp
synwire-mcp-server --project .

# Terminal 2: feature branch (different worktree, same repo)
cd ~/projects/myapp-feature
synwire-mcp-server --project .
```

Both connect to the same daemon. Indices are stored separately under `$XDG_CACHE_HOME/synwire/<repo_id>/<worktree_id>/`.

---

## Full configuration example

With auto-detection, a minimal config is usually sufficient:

```json
{
  "mcpServers": {
    "synwire": {
      "command": "synwire-mcp-server",
      "args": ["--project", "."]
    }
  }
}
```

For full control, all options can be specified explicitly:

```json
{
  "mcpServers": {
    "synwire": {
      "command": "synwire-mcp-server",
      "args": [
        "--project", ".",
        "--lsp", "rust-analyzer",
        "--dap", "codelldb",
        "--product-name", "myapp",
        "--embedding-model", "bge-small-en-v1.5",
        "--log-level", "info"
      ]
    }
  }
}
```

To disable auto-detection for one or both integrations:

```json
{
  "mcpServers": {
    "synwire": {
      "command": "synwire-mcp-server",
      "args": [
        "--project", ".",
        "--no-lsp",
        "--no-dap"
      ]
    }
  }
}
```

---

## What you learned

- LSP and DAP tools are **auto-detected** when `--project` is set
- `--lsp` and `--dap` override auto-detection with a specific server/adapter
- `--no-lsp` and `--no-dap` disable auto-detection and all related tools
- Polyglot repos detect multiple servers; the primary language is used for dispatch
- `meta.tool_search` and `meta.tool_list` provide namespace-based progressive tool discovery
- Agent skills are discovered from global and project-local directories
- The daemon manages shared state across multiple MCP server instances
- Multi-worktree projects share a `RepoId` but maintain separate indices

---

## See also

- [Tutorial 11: Getting Started with the MCP Server](./11-mcp-server.md) -- basic setup
- [Tutorial 12: Authoring Your First Agent Skill](./12-first-skill.md) -- skill authoring
- [How-To: LSP Integration](../how-to/lsp-integration.md) -- LSP tool details
- [How-To: DAP Integration](../how-to/dap-integration.md) -- DAP tool details
- [How-To: MCP Integration](../how-to/mcp-integration.md) -- MCP transport options
- [synwire-daemon](../explanation/synwire-daemon.md) -- daemon architecture

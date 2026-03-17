# How to: Integrate MCP Servers

## Using the standalone synwire-mcp-server

The easiest way to expose Synwire tools to an MCP host (Claude Code, GitHub Copilot, Cursor) is the standalone `synwire-mcp-server` binary.

### Install

```sh
cargo install synwire-mcp-server
```

### Configure Claude Code

Add to `.claude/mcp.json` in your project or home directory:

```json
{
  "synwire": {
    "command": "synwire-mcp-server",
    "args": ["--project", "."]
  }
}
```

With LSP support:

```json
{
  "synwire": {
    "command": "synwire-mcp-server",
    "args": ["--project", ".", "--lsp", "rust-analyzer"]
  }
}
```

### Config file (alternative to CLI flags)

Create `synwire.toml` in your project root:

```toml
project = "."
product_name = "myapp"
lsp = "rust-analyzer"
embedding_model = "bge-small-en-v1.5"
log_level = "info"
```

```json
{
  "synwire": {
    "command": "synwire-mcp-server",
    "args": ["--config", "synwire.toml"]
  }
}
```

CLI flags override config file values. See the [synwire-mcp-server](../explanation/synwire-mcp-server.md) explanation for all available flags.

---

**Goal:** Connect to Model Context Protocol servers via stdio, HTTP, or in-process transports, and manage multiple servers through `McpLifecycleManager`.

---

## Core trait: McpTransport

All transports implement:

```rust
pub trait McpTransport: Send + Sync {
    fn connect(&self)    -> BoxFuture<'_, Result<(), AgentError>>;
    fn reconnect(&self)  -> BoxFuture<'_, Result<(), AgentError>>;
    fn disconnect(&self) -> BoxFuture<'_, Result<(), AgentError>>;
    fn status(&self)     -> BoxFuture<'_, McpServerStatus>;
    fn list_tools(&self) -> BoxFuture<'_, Result<Vec<McpToolDescriptor>, AgentError>>;
    fn call_tool(&self, tool_name: &str, arguments: Value)
        -> BoxFuture<'_, Result<Value, AgentError>>;
}
```

`McpServerStatus` carries the server name, `McpConnectionState`, call success/failure counters, and an `enabled` flag.

`McpConnectionState` progression: `Disconnected → Connecting → Connected`. After a drop: `Connected → Reconnecting → Connected` (or back to `Disconnected` on failure). `Shutdown` is terminal.

---

## StdioMcpTransport

Manages a subprocess and exchanges newline-delimited JSON-RPC over its stdin/stdout.

```rust
use synwire_agent::mcp::stdio::StdioMcpTransport;
use synwire_core::mcp::traits::McpTransport;
use std::collections::HashMap;

let transport = StdioMcpTransport::new(
    "my-mcp-server",
    "npx",
    vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
    HashMap::from([
        ("MCP_WORKSPACE".to_string(), "/home/user/project".to_string()),
    ]),
);

transport.connect().await?;

let tools = transport.list_tools().await?;
for t in &tools {
    println!("{}: {}", t.name, t.description);
}

let result = transport.call_tool("read_file", serde_json::json!({
    "path": "/home/user/project/README.md"
})).await?;

transport.disconnect().await?;
```

Reconnecting kills the existing subprocess and spawns a new one:

```rust
transport.reconnect().await?;
```

---

## HttpMcpTransport

Connects to an HTTP-based MCP server. The `connect` call performs a health check by issuing `POST /tools/list`.

```rust
use synwire_agent::mcp::http::HttpMcpTransport;

let transport = HttpMcpTransport::new(
    "remote-mcp",
    "https://mcp.example.com",
    Some("Bearer sk-...".to_string()),
    Some(30),  // timeout_secs; None defaults to 30
);

transport.connect().await?;

let tools = transport.list_tools().await?;
let result = transport.call_tool("search", serde_json::json!({"query": "rust async"})).await?;
```

---

## InProcessMcpTransport

Registers native `Tool` implementations and exposes them via the `McpTransport` interface without any subprocess or network hop. Useful for built-in toolsets that benefit from the MCP lifecycle API.

```rust
use synwire_agent::mcp::in_process::InProcessMcpTransport;
use std::sync::Arc;

let mut transport = InProcessMcpTransport::new("builtin-tools");

// Register any type that implements `synwire_core::tools::Tool`.
transport.register(Arc::new(MyCustomTool)).await;

transport.connect().await?;  // transitions to Connected immediately

let tools = transport.list_tools().await?;
let result = transport.call_tool("my_tool", serde_json::json!({"input": "value"})).await?;
```

`reconnect` on an in-process transport is equivalent to `connect` — it simply marks the state as `Connected`.

---

## McpLifecycleManager

Manages multiple transports: connects all at start, monitors health, and reconnects dropped servers in the background.

```rust
use synwire_agent::mcp::lifecycle::McpLifecycleManager;
use std::sync::Arc;
use std::time::Duration;

let manager = Arc::new(McpLifecycleManager::new());

// Register servers with individual reconnect delays.
manager.register("filesystem", StdioMcpTransport::new(/* ... */), Duration::from_secs(5)).await;
manager.register("web-search",  HttpMcpTransport::new(/* ... */), Duration::from_secs(10)).await;

// Connect all enabled servers.
manager.start_all().await?;

// Spawn background health monitor (polls every 30 s, reconnects as needed).
Arc::clone(&manager).spawn_health_monitor(Duration::from_secs(30));

// Call a tool on a named server — auto-reconnects if needed.
let result = manager.call_tool("filesystem", "read_file", serde_json::json!({
    "path": "/project/src/main.rs"
})).await?;

// Inspect status of all servers.
let statuses = manager.all_status().await;
for s in &statuses {
    println!("{}: {:?} ok={} fail={}", s.name, s.state, s.calls_succeeded, s.calls_failed);
}

// Disable a server at runtime.
manager.disable("web-search").await?;

// Re-enable and reconnect.
manager.enable("web-search").await?;

// Shutdown.
manager.stop_all().await?;
```

Calling `call_tool` on a disabled server returns `AgentError::Backend` immediately without attempting to reconnect.

---

**See also**

- [How to: Configure the Middleware Stack](middleware.md)
- [How to: Add a Custom Tool](custom-tool.md)
- [Explanation: Architecture](../explanation/architecture.md)

# synwire-mcp-adapters

High-level MCP adapters for Synwire. Connects to multiple MCP servers simultaneously, aggregates their tools under a unified interface, and provides transport, validation, interceptors, and pagination out of the box.

## What this crate provides

- **`MultiServerMcpClient`** -- connects to N named MCP servers and routes tool calls to the correct server
- **`Connection`** -- transport configuration enum (Stdio, SSE, `StreamableHttp`, WebSocket)
- **`WebSocketMcpTransport`** -- WebSocket transport implementing `McpTransport`
- **`McpClientSession`** -- RAII session guard with drop-time cleanup
- **`McpToolProvider`** -- `ToolProvider` backed by `MultiServerMcpClient`
- **`ToolCallInterceptor`** -- onion-ordered middleware for tool call logging, rate limiting, or mutation
- **`validate_tool_arguments`** -- client-side JSON Schema validation before sending tool calls
- **`PaginationCursor`** -- cursor-based pagination with a 1000-page cap
- **`McpCallbacks`** -- logging, progress, and elicitation callback bundle
- **Bidirectional conversion** -- MCP tools to Synwire tools and back (`convert` module)
- **Zero unsafe code** -- `#![forbid(unsafe_code)]`

## Quick start

```toml
[dependencies]
synwire-mcp-adapters = "0.1"
```

Connect to two MCP servers and list aggregated tools:

```rust,no_run
use synwire_mcp_adapters::{MultiServerMcpClient, MultiServerMcpClientConfig, Connection};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut servers = HashMap::new();
    servers.insert("files".into(), Connection::Stdio {
        command: "mcp-file-server".into(),
        args: vec![],
        env: HashMap::new(),
    });
    servers.insert("search".into(), Connection::Sse {
        url: "http://localhost:8080/mcp".into(),
        auth_token: None,
        timeout_secs: Some(30),
    });

    let config = MultiServerMcpClientConfig { servers };
    let client = MultiServerMcpClient::connect(config).await?;

    for tool in client.list_tools().await? {
        println!("{}: {}", tool.name, tool.description);
    }
    Ok(())
}
```

## Documentation

- [MCP Integration Guide](https://randomvariable.github.io/synwire/how-to/mcp-integration.html)
- [Full API docs](https://docs.rs/synwire-mcp-adapters)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

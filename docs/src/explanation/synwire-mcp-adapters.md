# synwire-mcp-adapters: MCP Client Infrastructure

`synwire-mcp-adapters` provides the high-level client-side infrastructure for connecting to [Model Context Protocol](https://modelcontextprotocol.io) (MCP) servers. It handles multi-server aggregation, transport negotiation, tool conversion, argument validation, request interception, and session lifecycle.

## Why a separate crate?

MCP client logic is substantial --- transport management, tool schema conversion, interceptor chains, pagination, and validation. Keeping it separate from `synwire-core` (which defines the `McpTransport` trait) and `synwire-mcp-server` (which implements the server side) allows agents to connect to external MCP servers without pulling in server code, and vice versa.

## Key types

### `MultiServerMcpClient`

The central type. Connects to N named MCP servers simultaneously and aggregates their tools under a unified interface. Each server is identified by a string name and configured with a `Connection` variant.

```rust,no_run
use synwire_mcp_adapters::{MultiServerMcpClient, Connection, MultiServerMcpClientConfig};
use std::collections::HashMap;

let mut servers = HashMap::new();
servers.insert("filesystem".into(), Connection::Stdio {
    command: "npx".into(),
    args: vec!["-y".into(), "@anthropic/mcp-filesystem".into()],
});

let config = MultiServerMcpClientConfig { servers };
let client = MultiServerMcpClient::connect(config).await?;
```

Tools from all servers are merged into a single namespace. Each tool is tracked as an `AggregatedToolDescriptor` that records which server it came from, enabling correct dispatch when a tool is invoked.

### `Connection`

Transport configuration enum with four variants:

| Variant | Description |
|---|---|
| `Stdio` | Spawn a child process, communicate over stdin/stdout |
| `Sse` | Server-Sent Events over HTTP |
| `StreamableHttp` | Streamable HTTP (MCP 2025-03-26 transport) |
| `WebSocket` | WebSocket transport |

### `McpClientSession`

RAII session guard. On creation, it sends `initialize` and performs capability negotiation. On drop, it sends `shutdown`/`exit` and cleans up transport resources. This ensures that MCP server processes are not leaked even if the client panics.

### `ToolCallInterceptor`

Onion-ordered middleware for tool calls. Interceptors are executed in registration order before the call and in reverse order after it. Use cases include:

- **Logging** --- `LoggingInterceptor` records tool call timing and results
- **Rate limiting** --- throttle calls to a specific server
- **Caching** --- return cached results for deterministic tools
- **Approval gates** --- block calls pending human review

### `McpToolProvider`

Bridges MCP tools into Synwire's `ToolProvider` trait. Wraps a `MultiServerMcpClient` and presents all aggregated MCP tools as if they were native Synwire tools.

### `PaginationCursor`

Cursor-based pagination with a 1000-page safety cap. Used when listing tools or resources from MCP servers that return paginated results.

## Tool conversion

The `convert` module provides bidirectional conversion between MCP tool definitions and Synwire `ToolSchema`:

- **MCP to Synwire:** MCP tool schemas (JSON Schema with `name` + `description`) are converted to `ToolSchema` for use with `bind_tools`.
- **Synwire to MCP:** Synwire `ToolSchema` definitions are converted to MCP tool format for advertising in `tools/list` responses.

## Argument validation

`validate_tool_arguments` performs client-side JSON Schema validation of tool call arguments before sending them to the server. This catches malformed arguments early, providing better error messages than server-side validation and reducing unnecessary round-trips.

The validation uses the `jsonschema` crate for full JSON Schema Draft 2020-12 support.

## Callbacks

`McpCallbacks` bundles three callback traits:

| Trait | Purpose |
|---|---|
| `OnMcpLogging` | Receives log messages from MCP servers |
| `OnMcpProgress` | Receives progress notifications for long-running operations |

Built-in implementations include `DiscardLogging`, `DiscardProgress` (drop all), and `TracingLogging` (forward to `tracing`).

## Dependencies

| Crate | Role |
|---|---|
| `synwire-core` | `McpTransport`, `ToolProvider`, tool types |
| `synwire-agent` | Agent runtime types for tool dispatch |
| `tokio-tungstenite` | WebSocket transport implementation |
| `jsonschema` | Client-side argument validation |
| `futures-util` | Stream utilities for transport handling |

## Ecosystem position

```text
synwire-core       (McpTransport trait)
    |
    +-- synwire-mcp-adapters  (this crate: client, aggregation, conversion)
    |       |
    |       +-- synwire-mcp-server  (uses adapters for upstream MCP connections)
    |
    +-- synwire-agent         (McpTransport implementations: stdio, HTTP, in-process)
```

## See also

- [MCP Integration](../how-to/mcp-integration.md) --- how-to guide
- [synwire-mcp-server](./synwire-mcp-server.md) --- the server side of MCP
- [synwire-core: Trait Contract Layer](./synwire-core.md) --- `McpTransport` trait

# MCP Adapters & Tool System

## Overview

The `synwire-mcp-adapters` crate provides bidirectional integration between MCP (Model Context Protocol) servers and Synwire's tool system. Built on the `rmcp` SDK, it enables agents to discover and invoke tools hosted by external MCP servers, and conversely exposes Synwire tools to MCP clients. This is an M2 feature.

## Multi-Server Client

The primary entry point is `MultiServerMcpClient` (FR-112), which manages connections to multiple MCP servers simultaneously (FR-113). It accepts a connections map, callbacks, interceptors, and a `tool_name_prefix` flag. All operations are async-only, and connections to multiple servers are established simultaneously (FR-131).

## Transport Layer

### Connection Variants

The `Connection` enum (FR-118) supports four transport variants:

- **Stdio** -- spawns a child process, communicating over stdin/stdout. Configured with `command`, `args`, `env`, and `cwd`.
- **SSE** -- legacy Server-Sent Events transport. Configured with `url`, `headers`, and `timeout`.
- **StreamableHttp** -- the current MCP specification transport. Configured with `url`, `headers`, `timeout`, and `sse_read_timeout`.
- **WebSocket** -- WebSocket-based transport. Configured with `url` and `headers`.

### Session Management

`create_session()` accepts any `Connection` variant and returns an `McpClientSession` (FR-119). Session lifecycle is managed through guard-based cleanup (FR-117), ensuring connections are properly torn down even on early returns or panics. Both per-transport and per-tool timeouts are supported (FR-131).

HTTP client behaviour can be customised by implementing rmcp's `StreamableHttpClient` trait (FR-129). Authentication supports Bearer and Basic schemes.

## Tool Operations

### MCP to Synwire (FR-114)

`get_tools()` loads tools from one or all connected servers, returning `Vec<Box<dyn Tool>>` (FR-114). Tool listing uses cursor-based pagination with a safeguard that caps iteration at 1000 pages (FR-120).

`convert_mcp_tool_to_synwire_tool()` maps an MCP tool definition to a Synwire `Tool`, returning `(content, artifact)` with MCP annotations carried through as metadata (FR-121). When the `tool_name_prefix` flag is set, tool names are prefixed as `{server_name}_{tool_name}`, with server names sanitised for valid identifiers (FR-130).

### Synwire to MCP (FR-122)

`to_mcp_tool()` performs the reverse conversion, exposing a Synwire tool as an MCP tool definition. It validates the `args_schema` and returns an error if injected arguments are present.

### Content Type Conversion (FR-123)

Content types are mapped between MCP and Synwire representations:

- **Text**, **Image**, **ResourceLink**, and **EmbeddedResource** convert directly.
- **AudioContent** returns `UnsupportedContent`, as Synwire does not yet model audio.
- When MCP sets the `isError` flag, the conversion raises a `ToolException`.

## Resource & Prompt Operations

### Resources (FR-116, FR-124)

`get_resources()` loads MCP resources as `McpBlob` equivalents, excluding dynamic resources (FR-116). Supporting functions include `convert_mcp_resource_to_blob()`, `load_mcp_resources()`, and `get_mcp_resource()` for individual resource retrieval (FR-124).

### Prompts (FR-115, FR-125)

`get_prompt()` retrieves an MCP prompt and converts it to Synwire `Message` types (FR-115). `convert_mcp_prompt_message()` handles role-based mapping and multi-content support, translating MCP's prompt message structure into Synwire's message model (FR-125).

## Interceptors

The `ToolCallInterceptor` trait (FR-126) follows an onion/middleware pattern, allowing multiple interceptors to wrap tool invocations in composable layers. Interceptors are panic-safe, ensuring that a failing interceptor does not corrupt the call chain.

Each interceptor receives an `McpToolCallRequest` (FR-127) containing the tool `name`, `args`, `server_name`, `headers`, and runtime context. Interceptors may return an `McpToolCallResult`, which is a union of `CallToolResult`, `ToolMessage`, or `Command`. The chain executes in correct onion order (SC-024).

## Callbacks (FR-128)

`McpCallbacks` provides slots for three callback types:

- **LoggingMessage** -- server-side log output.
- **Progress** -- progress notifications for long-running operations.
- **Elicitation** -- interactive prompts from the server requesting user input.

These are separate from Synwire's `CallbackHandler` and are scoped to the MCP transport layer.

## JSON Schema Handling (FR-132)

MCP tool schemas are represented as `serde_json::Value`, with schema validation performed before tool invocation. This avoids the need for a full JSON Schema type system while still catching malformed arguments early.

## Tool System Enrichment

Beyond MCP-specific adapters, the broader tool system provides several capabilities that underpin tool management across Synwire.

### Tool Classification and Output

`ToolCategory` (FR-333) classifies tools as `Builtin`, `Custom`, `Mcp`, `Remote`, or `WorkflowAsTool`. `ToolOutput` is extended with `content_type: ToolContentType` (FR-334), where `ToolContentType` (FR-362) defines variants for `Text`, `Image`, `File`, and `Json`.

### Tool Providers (FR-335)

The `ToolProvider` trait exposes `discover_tools()` and `get_tool()` methods, with three built-in implementations:

- **`StaticToolProvider`** -- a fixed set of tools configured at construction.
- **`McpToolProvider`** -- backed by `MultiServerMcpClient`.
- **`CompositeToolProvider`** -- aggregates multiple providers into a single interface.

### Operational Controls

- **Timeouts** (FR-357) -- per-tool timeout with `timeout_behavior` set to either `ReturnError` or `RaiseException`.
- **Enablement predicates** (FR-358) -- `is_enabled` controls whether a tool is included in the LLM schema. Disabled tools are omitted entirely.
- **Usage limits** (FR-359) -- `max_usage_count` caps how many times a tool may be invoked, returning `ToolUsageLimitExceeded` when exceeded.
- **Name validation** (FR-360) -- tool names must match `^[a-zA-Z0-9_-]{1,64}$`, enforced at construction time.
- **Result truncation** (FR-347) -- `ToolNode` truncates results exceeding `max_result_size` (default 100 KB).
- **Argument validation** (FR-349) -- tool argument schemas are validated before invocation.

### Tool Kind (FR-101)

`ToolKind` classifies tools by their operational nature: `read`, `edit`, `search`, `execute`, or `other`. This is primarily intended for permission UIs that need to communicate the impact of tool invocations to users.

### Proc-Macro (FR-361)

The `#[tool]` proc-macro generates a `Tool` implementation from an async function, reducing boilerplate for custom tool definitions (SC-057).

## Agents as Tools

`CompiledGraph::as_tool()` (FR-308) wraps a compiled graph as a `Tool`, enabling graph-in-graph composition. A `CompiledGraph` can also be used directly as a node within another `StateGraph` (FR-309), allowing hierarchical agent architectures.

## Error Types

The MCP adapters crate defines the following error variants:

| Error | Description |
|---|---|
| `ServerNotFound` | Unknown server name passed to a multi-server operation |
| `Transport` | Connection or protocol-level errors |
| `ConnectionFailed` | Initial connection establishment failure |
| `Timeout` | Operation exceeded its configured timeout |
| `ToolNotFound` | Requested tool name not found on the target server |
| `SchemaValidation` | Tool arguments failed schema validation |

## Success Criteria

| ID | Criterion |
|---|---|
| SC-021 | `MultiServerMcpClient` connects to two MCP servers (stdio + HTTP) and loads tools for an agent |
| SC-022 | A Synwire tool round-trips through MCP conversion without data loss |
| SC-023 | Each MCP transport variant passes connection and tool call tests |
| SC-024 | Interceptor chain executes in correct onion order |
| SC-025 | All three MCP callback types deliver notifications with correct context |
| SC-057 | `#[tool]` macro generates a working tool from an async function |

## Research Findings

All 103 parity items (CHK506--CHK608) from the mcp-adapters-parity research have been resolved. Key architectural decisions:

- **rmcp SDK** is used rather than a custom MCP implementation. rmcp types are re-exported for ergonomics.
- **Interceptor pattern** was chosen over simple hook callbacks for composability and middleware-style layering.
- **Async-only** design (no sync variants) aligns with MCP's inherently async nature.

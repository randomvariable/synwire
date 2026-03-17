# Trait Contracts: Agent Core Runtime

**Feature**: 003-agent-core | **Date**: 2026-03-15 (expanded 2026-03-16, MCP adapters 2026-03-16)

This document defines the public trait interfaces exposed by the agent core runtime. These are the contracts that users and implementations must satisfy.

## synwire-core: Core Traits

### DirectivePayload

```rust
/// User-defined directive payload for Custom variant.
/// Must be registered with `#[typetag::serde]` for serialization.
#[typetag::serde]
pub trait DirectivePayload: Send + Sync + Debug + DynClone {
    fn name(&self) -> &str;
}
```

### DirectiveExecutor

```rust
/// Executes directives produced by agent nodes.
/// Implementations control side effect execution.
pub trait DirectiveExecutor: Send + Sync {
    fn execute_directive<'a>(
        &'a self,
        directive: &'a Directive,
    ) -> BoxFuture<'a, Result<Option<serde_json::Value>, DirectiveError>>;
}
```

### DirectiveFilter

```rust
/// Inspects, transforms, or suppresses directives before execution.
pub trait DirectiveFilter: Send + Sync {
    fn filter(&self, directive: Directive) -> Option<Directive>;
    fn name(&self) -> &str;
}
```

### ExecutionStrategy

```rust
/// Controls how agent actions are orchestrated.
pub trait ExecutionStrategy: Send + Sync {
    fn execute<'a>(
        &'a self,
        action: &'a str,
        input: serde_json::Value,
    ) -> BoxFuture<'a, Result<serde_json::Value, StrategyError>>;

    fn tick(&self) -> BoxFuture<'_, Result<Option<serde_json::Value>, StrategyError>>;

    fn snapshot(&self) -> Result<StrategySnapshot, StrategyError>;

    fn signal_routes(&self) -> Vec<SignalRoute> { vec![] }
}
```

### GuardCondition

```rust
/// Runtime predicate for FSM transition guards.
pub trait GuardCondition: Send + Sync {
    fn check(&self, context: &GuardContext) -> bool;
    fn name(&self) -> &str;
}
```

### PluginStateKey

```rust
/// Declares a plugin's state type and serialization key.
pub trait PluginStateKey: 'static + Send + Sync {
    type State: Send + Sync + Default + Serialize + DeserializeOwned + Clone + Debug;
    const KEY: &'static str;
}
```

### Plugin

```rust
/// Runner-scoped plugin with lifecycle hooks.
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_user_message<'a>(&'a self, msg: &'a Message) -> BoxFuture<'a, Result<(), PluginError>> {
        Box::pin(async { Ok(()) })
    }
    fn on_event<'a>(&'a self, event: &'a AgentEvent) -> BoxFuture<'a, Result<(), PluginError>> {
        Box::pin(async { Ok(()) })
    }
    fn before_run<'a>(&'a self, ctx: &'a RunContext<()>) -> BoxFuture<'a, Result<(), PluginError>> {
        Box::pin(async { Ok(()) })
    }
    fn after_run<'a>(&'a self, ctx: &'a RunContext<()>, result: &'a AgentResult) -> BoxFuture<'a, Result<(), PluginError>> {
        Box::pin(async { Ok(()) })
    }
    fn signal_routes(&self) -> Vec<SignalRoute> { vec![] }
}
```

### SignalRouter

```rust
/// Routes signals to actions based on signal kind and predicate.
pub trait SignalRouter: Send + Sync {
    fn route(&self, signal: &Signal) -> Option<Action>;
}
```

### Vfs

```rust
/// Pluggable interface for file, shell, and system operations.
/// All async methods return BoxFuture for dyn-safety.
/// Providers declare capabilities via VfsCapabilities bitflags.
/// Unsupported ops return VfsError::Unsupported.
pub trait Vfs: Send + Sync {
    fn ls(&self, path: &str, opts: &LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>>;
    fn read(&self, path: &str, opts: &ReadOptions) -> BoxFuture<'_, Result<FileContent, VfsError>>;
    fn write(&self, path: &str, content: &[u8], opts: &WriteOptions) -> BoxFuture<'_, Result<WriteResult, VfsError>>;
    fn edit(&self, path: &str, edits: &[EditOp]) -> BoxFuture<'_, Result<EditResult, VfsError>>;
    fn grep(&self, pattern: &str, opts: &GrepOptions) -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>>;
    fn glob(&self, pattern: &str, opts: &GlobOptions) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>>;
    fn upload(&self, local: &str, remote: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>>;
    fn download(&self, remote: &str, local: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>>;
    fn pwd(&self) -> Result<String, VfsError>;
    fn cd(&self, path: &str) -> Result<(), VfsError>;
    fn rm(&self, path: &str, opts: &RmOptions) -> BoxFuture<'_, Result<(), VfsError>>;
    fn cp(&self, src: &str, dst: &str, opts: &CpOptions) -> BoxFuture<'_, Result<(), VfsError>>;
    fn mv_file(&self, src: &str, dst: &str) -> BoxFuture<'_, Result<(), VfsError>>;
    fn tree(&self, path: &str, opts: &TreeOptions) -> BoxFuture<'_, Result<TreeOutput, VfsError>>;
    fn head(&self, path: &str, lines: usize) -> BoxFuture<'_, Result<FileContent, VfsError>>;
    fn tail(&self, path: &str, lines: usize) -> BoxFuture<'_, Result<FileContent, VfsError>>;
    fn stat(&self, path: &str) -> BoxFuture<'_, Result<FileStat, VfsError>>;
    fn wc(&self, path: &str) -> BoxFuture<'_, Result<WordCount, VfsError>>;
    fn du(&self, path: &str) -> BoxFuture<'_, Result<DiskUsage, VfsError>>;
    fn diff(&self, a: &str, b: &str) -> BoxFuture<'_, Result<DiffOutput, VfsError>>;
    fn find(&self, path: &str, opts: &FindOptions) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>>;
    fn mkdir(&self, path: &str, recursive: bool) -> BoxFuture<'_, Result<(), VfsError>>;
    fn touch(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>>;
    fn append(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<(), VfsError>>;
    fn ln(&self, target: &str, link: &str, symbolic: bool) -> BoxFuture<'_, Result<(), VfsError>>;
    fn chmod(&self, path: &str, mode: u32) -> BoxFuture<'_, Result<(), VfsError>>;
    fn watch(&self, path: &str) -> BoxFuture<'_, Result<BoxStream<'_, FsEvent>, VfsError>>;
    fn check_stale(&self, path: &str, known_mtime: SystemTime) -> BoxFuture<'_, Result<bool, VfsError>>;
    fn index(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>>;
    fn index_status(&self, path: &str) -> BoxFuture<'_, Result<IndexStatus, VfsError>>;
    fn semantic_search(&self, query: &str, opts: &SemanticSearchOptions) -> BoxFuture<'_, Result<Vec<SemanticMatch>, VfsError>>;
    fn skeleton(&self, path: &str, opts: &SkeletonOptions) -> BoxFuture<'_, Result<SkeletonOutput, VfsError>>;
    fn capabilities(&self) -> VfsCapabilities;
}
```

### SandboxProtocol

```rust
/// Extends Vfs with shell execution capabilities.
pub trait SandboxProtocol: Vfs {
    fn execute<'a>(&'a self, command: &'a str, opts: &'a ExecuteOptions) -> BoxFuture<'a, Result<ExecuteResponse, VfsError>>;
    fn execute_pipeline<'a>(&'a self, pipeline: &'a Pipeline) -> BoxFuture<'a, Result<ExecuteResponse, VfsError>>;
    fn id(&self) -> &str;
}
```

### ApprovalCallback

```rust
/// Callback for human-in-the-loop approval of risky operations.
/// Applies to all tool invocations when a permission rule/mode requires approval.
pub trait ApprovalCallback: Send + Sync {
    fn request_approval<'a>(
        &'a self,
        req: &'a ApprovalRequest,
    ) -> BoxFuture<'a, Result<ApprovalDecision, VfsError>>;
}

/// Decision variants: Allow, Deny, AllowAlways (persistent), Abort (stop agent).
/// Allow and AllowAlways may include modified tool input.
```

### Middleware

```rust
/// Stackable middleware for cross-cutting agent concerns.
pub trait Middleware: Send + Sync {
    fn name(&self) -> &str;
    fn process<'a>(
        &'a self,
        input: MiddlewareInput<'a>,
        next: MiddlewareNext<'a>,
    ) -> BoxFuture<'a, Result<MiddlewareResult, MiddlewareError>>;
    fn tools(&self) -> Vec<Box<dyn Tool>> { vec![] }
    fn system_prompt_additions(&self) -> Option<String> { None }
}
```

### AgentNode

```rust
/// Defines an agent node in the runtime.
pub trait AgentNode: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run<'a>(
        &'a self,
        input: serde_json::Value,
        ctx: &'a RunContext<()>,
    ) -> BoxStream<'a, Result<AgentEvent, AgentError>>;
    fn sub_agents(&self) -> Vec<Arc<dyn AgentNode>> { vec![] }
}
```

### BeforeAgentCallback / AfterAgentCallback

```rust
pub trait BeforeAgentCallback: Send + Sync {
    fn before<'a>(&'a self, ctx: &'a RunContext<()>) -> BoxFuture<'a, Result<(), AgentError>>;
}

pub trait AfterAgentCallback: Send + Sync {
    fn after<'a>(&'a self, ctx: &'a RunContext<()>, result: &'a AgentResult) -> BoxFuture<'a, Result<(), AgentError>>;
}
```

### OnModelErrorCallback

```rust
pub trait OnModelErrorCallback: Send + Sync {
    fn on_error<'a>(
        &'a self,
        error: &'a AgentError,
    ) -> BoxFuture<'a, Option<serde_json::Value>>;
}
```

### HookCallback

```rust
/// Lifecycle hook callback invoked at specific agent lifecycle points.
pub trait HookCallback: Send + Sync {
    fn invoke<'a>(
        &'a self,
        input: HookInput<'a>,
        signal: &'a AbortSignal,
    ) -> BoxFuture<'a, Result<HookOutput, HookError>>;
}
```

### PreToolUseHook

```rust
/// Pre-tool-use hook with approve/reject/modify semantics.
/// Output variants:
///   Approve — proceed with original args
///   Reject(reason) — return error to model
///   Modify(new_args) — proceed with modified args
pub trait PreToolUseHook: Send + Sync {
    fn before_tool<'a>(
        &'a self,
        tool_name: &'a str,
        args: &'a serde_json::Value,
        signal: &'a AbortSignal,
    ) -> BoxFuture<'a, Result<PreToolDecision, HookError>>;
}
```

### SessionManager

```rust
/// Session persistence and lifecycle management.
pub trait SessionManager: Send + Sync {
    fn list_sessions<'a>(
        &'a self,
        filter: &'a SessionFilter,
    ) -> BoxFuture<'a, Result<Vec<SessionMetadata>, SessionError>>;

    fn resume_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, Result<Session, SessionError>>;

    fn delete_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, Result<(), SessionError>>;

    fn fork_session<'a>(
        &'a self,
        session_id: &'a str,
        at_message: &'a str,
    ) -> BoxFuture<'a, Result<Session, SessionError>>;

    fn rewind_to<'a>(
        &'a self,
        session_id: &'a str,
        message_id: &'a str,
    ) -> BoxFuture<'a, Result<RewindResult, SessionError>>;

    fn tag_session<'a>(
        &'a self,
        session_id: &'a str,
        tag: &'a str,
    ) -> BoxFuture<'a, Result<(), SessionError>>;

    fn rename_session<'a>(
        &'a self,
        session_id: &'a str,
        title: &'a str,
    ) -> BoxFuture<'a, Result<(), SessionError>>;
}
```

### McpTransport

```rust
/// Transport layer for MCP server communication.
pub trait McpTransport: Send + Sync {
    fn connect<'a>(&'a self) -> BoxFuture<'a, Result<(), McpError>>;

    fn reconnect<'a>(&'a self) -> BoxFuture<'a, Result<(), McpError>>;

    fn status(&self) -> McpConnectionState;

    fn list_tools<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<McpToolInfo>, McpError>>;

    fn call_tool<'a>(
        &'a self,
        name: &'a str,
        args: serde_json::Value,
    ) -> BoxFuture<'a, Result<serde_json::Value, McpError>>;

    fn disconnect<'a>(&'a self) -> BoxFuture<'a, Result<(), McpError>>;
}
```

### OnElicitation

```rust
/// Callback for MCP servers requesting user input through the agent host.
pub trait OnElicitation: Send + Sync {
    fn elicit<'a>(
        &'a self,
        request: &'a ElicitationRequest,
        signal: &'a AbortSignal,
    ) -> BoxFuture<'a, Result<ElicitationResult, ElicitationError>>;
}
```

### SamplingProvider

```rust
/// Abstraction for tool-internal LLM access.
/// Two implementations: MCP sampling (delegates to client) and direct model invocation.
/// All calls are lazy/on-demand — zero calls during indexing.
pub trait SamplingProvider: Send + Sync {
    fn sample<'a>(
        &'a self,
        messages: &'a [Message],
        max_tokens: u32,
    ) -> BoxFuture<'a, Result<String, SamplingError>>;

    fn is_available(&self) -> bool;
}
```

### ToolSearchIndex

```rust
/// Framework-level progressive tool discovery.
/// Provides embedding-based retrieval + keyword boosting + namespace browsing.
pub trait ToolSearchProvider: Send + Sync {
    fn search<'a>(
        &'a self,
        query: &'a str,
        top_k: usize,
    ) -> BoxFuture<'a, Result<Vec<ToolSearchResult>, ToolSearchError>>;

    fn list_namespaces(&self) -> Vec<String>;

    fn list_tools(&self, namespace: Option<&str>) -> Vec<ToolSummary>;

    fn get_schema(&self, tool_name: &str) -> Option<ToolSchema>;

    fn mark_loaded(&self, tool_name: &str);
}
```

### SkillRuntime

```rust
/// Execution backend for agent skills.
pub trait SkillRuntime: Send + Sync {
    fn name(&self) -> &str;

    fn execute<'a>(
        &'a self,
        entrypoint: &'a str,
        args: serde_json::Value,
        vfs: &'a dyn Vfs,
    ) -> BoxFuture<'a, Result<ToolOutput, SkillError>>;

    fn is_available(&self) -> bool;
}
```

### StorageMigration

```rust
/// Per-subsystem schema migration.
pub trait StorageMigration: Send + Sync {
    fn current_version(&self) -> u32;

    fn migrate<'a>(
        &'a self,
        from: u32,
        to: u32,
        path: &'a Path,
    ) -> BoxFuture<'a, Result<(), MigrationError>>;
}
```

### ModelProvider

```rust
/// Provides model enumeration and capability metadata.
pub trait ModelProvider: Send + Sync {
    fn list_models<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<ModelInfo>, ModelError>>;
}
```

## synwire-agent: Implementation Contracts

Backend implementations in `synwire-agent` must pass the Vfs conformance test suite defined in `synwire-test-utils`. The conformance suite tests:

1. All capability-advertised operations succeed
2. Operations not in capabilities return `VfsError::Unsupported`
3. Path traversal protection (rejected with `VfsError::PathTraversal`)
4. Working directory persistence across operations
5. Concurrent access safety (`Send + Sync`)
6. Error code correctness for all `VfsError` variants

MCP transport implementations must pass the MCP transport conformance suite:

1. Connect/disconnect lifecycle completes without errors
2. Reconnect recovers from transient failures
3. Tool listing returns valid tool definitions
4. Tool invocation returns results matching tool schemas
5. Timeout is enforced on all operations

Session manager implementations must pass the session conformance suite:

1. Create → list → resume round-trip preserves all state
2. Fork creates independent copy with shared history prefix
3. Rewind restores state and reports affected files
4. Delete removes all session data permanently
5. Tag/rename operations persist across resume

**Semantic index conformance**:

1. Index a directory → search returns relevant results
2. Incremental re-index skips unchanged files
3. File watcher triggers re-index on change
4. `IndexNotReady` returned before indexing completes

**Tool search conformance**:

1. `tool_search` returns relevant tools for natural language queries
2. Namespace browsing returns correct tool sets
3. Schema deduplication avoids re-sending loaded schemas
4. Hybrid scoring ranks exact name matches above semantic matches

**Skill runtime conformance**:

1. Lua skill executes and returns `ToolOutput`
2. Rhai skill executes and returns `ToolOutput`
3. WASM skill runs sandboxed (denied capabilities rejected)
4. External skill emits warning
5. Instruction/operation limits enforced

## synwire-mcp-adapters: MCP Adapter Contracts

### ToolCallInterceptor

```rust
/// Composable onion/middleware for MCP tool calls.
/// Interceptors wrap tool invocations in layers.
/// Panic-safe: a failing interceptor does not corrupt the chain.
pub trait ToolCallInterceptor: Send + Sync {
    fn intercept<'a>(
        &'a self,
        request: McpToolCallRequest,
        next: InterceptorNext<'a>,
    ) -> BoxFuture<'a, Result<McpToolCallResult, McpAdapterError>>;
}
```

### ToolProvider

```rust
/// Trait for discovering and retrieving tools from heterogeneous sources.
/// Implementations: StaticToolProvider, McpToolProvider, CompositeToolProvider.
/// All implementations must be Send + Sync.
pub trait ToolProvider: Send + Sync {
    fn discover_tools<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<Box<dyn Tool>>, ToolProviderError>>;

    fn get_tool<'a>(
        &'a self,
        name: &'a str,
    ) -> BoxFuture<'a, Result<Option<Box<dyn Tool>>, ToolProviderError>>;
}
```

## synwire-mcp-adapters: Conformance Requirements

MCP adapter implementations must pass the following conformance tests:

**MultiServerMcpClient conformance**:

1. Simultaneous connection to 2+ servers completes without sequential blocking
2. Tool aggregation returns tools from all connected servers
3. Tool name prefixing produces `{server}_{tool}` format with sanitised names
4. Health monitoring detects disconnected servers and excludes their tools
5. Cursor pagination terminates at 1000-page cap with misbehaving servers

**Tool conversion conformance**:

1. MCP tool → Synwire tool round-trip preserves name, description, schema, annotations
2. Synwire tool → MCP tool rejects tools with injected arguments
3. Content type mapping: Text, Image, ResourceLink, EmbeddedResource convert correctly
4. AudioContent returns `UnsupportedContent`
5. `isError` flag on MCP result raises `ToolException`

**Interceptor conformance**:

1. 3 interceptors execute in correct onion order (A→B→C→tool→C→B→A)
2. Short-circuit interceptor skips inner chain and tool
3. Panicking interceptor is caught and converted to error without corrupting chain

**ToolProvider conformance**:

1. `StaticToolProvider` returns exactly the configured tools
2. `McpToolProvider` returns tools from connected MCP servers
3. `CompositeToolProvider` aggregates tools from all sub-providers
4. Name collisions across providers are handled per configured policy

**Tool operational controls conformance**:

1. Timeout cancels tool execution within configured duration
2. Usage limit returns `ToolUsageLimitExceeded` after N invocations
3. Disabled tools are excluded from LLM schema
4. Invalid tool names are rejected at construction
5. Results exceeding max_result_size are truncated
6. Arguments failing JSON Schema validation are rejected before invocation

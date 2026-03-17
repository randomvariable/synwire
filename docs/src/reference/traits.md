# Traits Reference

All public traits in `synwire-core` and `synwire-agent`. Every trait is
`Send + Sync` unless noted otherwise. Methods returning async results use
`BoxFuture<'_, Result<T, E>>` from `synwire_core::BoxFuture`.

---

## Core Agent Traits — `synwire_core::agents`

---

### `AgentNode`

```rust
pub trait AgentNode: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run(&self, input: Value) -> BoxFuture<'_, Result<AgentEventStream, AgentError>>;
    fn sub_agents(&self) -> Vec<String> { vec![] }  // default: empty
}
```

A runnable agent that produces a stream of [`AgentEvent`](types.md#agentevent) values.

| Method | Description |
|--------|-------------|
| `name` | Stable identifier used for routing, logging, and sub-agent references. |
| `description` | Human-readable string; surfaced in introspection and UI. |
| `run` | Start a turn. Returns an event stream that terminates with `TurnComplete` or `Error`. |
| `sub_agents` | Names of agents this node may spawn. Used by the runner for capability declaration. |

The `Agent<O>` builder implements `AgentNode`. Provider crates replace the
stub model invocation with a real LLM call.

---

### `ExecutionStrategy`

```rust
pub trait ExecutionStrategy: Send + Sync {
    fn execute<'a>(&'a self, action: &'a str, input: Value)
        -> BoxFuture<'a, Result<Value, StrategyError>>;
    fn tick(&self)
        -> BoxFuture<'_, Result<Option<Value>, StrategyError>>;
    fn snapshot(&self)
        -> Result<Box<dyn StrategySnapshot>, StrategyError>;
    fn signal_routes(&self) -> Vec<SignalRoute> { vec![] } // default: empty
}
```

Controls how an agent orchestrates actions.

| Method | Description |
|--------|-------------|
| `execute` | Attempt an action from the current state. Returns the (possibly modified) input on success. `FsmStrategy` validates the action against the current FSM state. `DirectStrategy` passes input through unconditionally. |
| `tick` | Process pending deferred work. Returns `Some(value)` if there is a result to route back; `None` otherwise. |
| `snapshot` | Capture serialisable strategy state for checkpointing. |
| `signal_routes` | Signal routes contributed by this strategy to the composed router. |

---

### `GuardCondition`

```rust
pub trait GuardCondition: Send + Sync {
    fn evaluate(&self, input: &Value) -> bool;
    fn name(&self) -> &str;
}
```

Predicate evaluated before an FSM transition is accepted.

| Method | Description |
|--------|-------------|
| `evaluate` | Return `true` to allow the transition. |
| `name` | Human-readable name used in error messages and logging. |

`ClosureGuard` is a convenience adapter that wraps `Fn(&Value) -> bool`.

---

### `StrategySnapshot`

```rust
pub trait StrategySnapshot: Send + Sync {
    fn to_value(&self) -> Result<Value, StrategyError>;
}
```

Serialises strategy state to JSON for checkpointing. The value is opaque from
the runtime's perspective; each strategy defines its own schema.

---

### `DirectiveFilter`

```rust
pub trait DirectiveFilter: Send + Sync {
    fn filter(&self, directive: Directive) -> Option<Directive>;
    fn decision(&self, directive: &Directive) -> FilterDecision { /* default */ }
}
```

Filters directives before they reach the executor.

| Method | Description |
|--------|-------------|
| `filter` | Return `Some(directive)` to pass through (possibly modified) or `None` to suppress. |
| `decision` | Inspect a directive without consuming it. Default implementation calls `filter` on a clone. |

`FilterChain` applies a sequence of filters in registration order; the first
`None` result short-circuits the chain.

---

### `DirectiveExecutor`

```rust
pub trait DirectiveExecutor: Send + Sync {
    fn execute_directive(
        &self,
        directive: &Directive,
    ) -> BoxFuture<'_, Result<Option<Value>, DirectiveError>>;
}
```

Executes a [`Directive`](types.md#directive) and optionally routes a result value back to
the agent.

| Returns | Meaning |
|---------|---------|
| `Ok(None)` | Directive executed; no result to route back. |
| `Ok(Some(v))` | Result value to inject into the next agent turn (used by `RunInstruction`). |
| `Err(e)` | Execution failed. |

`NoOpExecutor` always returns `Ok(None)`. Useful for pure directive-testing
without side effects.

---

### `DirectivePayload`

```rust
#[typetag::serde(tag = "custom_type")]
pub trait DirectivePayload: Debug + Send + Sync + DynClone {}
```

Marker trait for user-defined directive data carried by `Directive::Custom`.
Requires `#[typetag::serde]` on the implementation for serialisation support.

---

### `Middleware`

```rust
pub trait Middleware: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, input: MiddlewareInput)
        -> BoxFuture<'_, Result<MiddlewareResult, AgentError>>;  // default: pass-through
    fn tools(&self) -> Vec<Box<dyn Tool>> { vec![] }
    fn system_prompt_additions(&self) -> Vec<String> { vec![] }
}
```

Cross-cutting concern injected into the agent loop via `MiddlewareStack`.

| Method | Description |
|--------|-------------|
| `name` | Identifier for logging and ordering diagnostics. |
| `process` | Transform `MiddlewareInput`. Return `Continue(modified)` to chain or `Terminate(reason)` to halt. The default implementation is a no-op pass-through. |
| `tools` | Additional tools injected into the agent context by this middleware. |
| `system_prompt_additions` | Prompt fragments appended in stack order by `MiddlewareStack::system_prompt_additions`. |

---

### `Plugin`

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_user_message<'a>(&'a self, input: &'a PluginInput, state: &'a PluginStateMap)
        -> BoxFuture<'a, Vec<Directive>> { /* default: empty */ }
    fn on_event<'a>(&'a self, event: &'a AgentEvent, state: &'a PluginStateMap)
        -> BoxFuture<'a, Vec<Directive>> { /* default: empty */ }
    fn before_run<'a>(&'a self, state: &'a PluginStateMap)
        -> BoxFuture<'a, Vec<Directive>> { /* default: empty */ }
    fn after_run<'a>(&'a self, state: &'a PluginStateMap)
        -> BoxFuture<'a, Vec<Directive>> { /* default: empty */ }
    fn signal_routes(&self) -> Vec<SignalRoute> { vec![] }
}
```

Lifecycle extension point for the agent loop. All methods have default no-op
implementations; plugins only override hooks they require.

| Method | Triggered when |
|--------|----------------|
| `on_user_message` | A user message arrives. |
| `on_event` | Any `AgentEvent` is emitted. |
| `before_run` | Before each run loop iteration. |
| `after_run` | After each run loop iteration. |
| `signal_routes` | Called at startup to register signal routes in the composed router. |

Plugins return `Vec<Directive>` to request effects without direct mutation.

---

### `PluginStateKey`

```rust
pub trait PluginStateKey: Send + Sync + 'static {
    type State: Send + Sync + 'static;
    const KEY: &'static str;
}
```

Typed key for isolated plugin state stored in [`PluginStateMap`](types.md#pluginstatemap).

| Associated item | Description |
|-----------------|-------------|
| `type State` | The concrete state type stored for this plugin. Must be `Send + Sync + 'static`. |
| `const KEY` | Unique string key used for serialisation. Must be globally unique across all registered plugins. |

Plugins cannot access other plugins' state — the `TypeId` of `P` enforces
isolation at runtime.

---

### `SignalRouter`

```rust
pub trait SignalRouter: Send + Sync {
    fn route(&self, signal: &Signal) -> Option<Action>;
    fn routes(&self) -> Vec<SignalRoute>;
}
```

Routes [`Signal`](types.md#signal) values to [`Action`](types.md#action) decisions.

| Method | Description |
|--------|-------------|
| `route` | Return the best-matching action, or `None` if no route matches. |
| `routes` | All routes contributed by this router. |

`ComposedRouter` merges strategy, agent, and plugin route tiers: strategy routes
always win regardless of priority value. Within a tier, the highest `priority`
field wins.

---

### `SessionManager`

```rust
pub trait SessionManager: Send + Sync {
    fn list(&self)
        -> BoxFuture<'_, Result<Vec<SessionMetadata>, AgentError>>;
    fn resume(&self, session_id: &str)
        -> BoxFuture<'_, Result<Session, AgentError>>;
    fn save(&self, session: &Session)
        -> BoxFuture<'_, Result<(), AgentError>>;
    fn delete(&self, session_id: &str)
        -> BoxFuture<'_, Result<(), AgentError>>;
    fn fork(&self, session_id: &str, new_name: Option<String>)
        -> BoxFuture<'_, Result<SessionMetadata, AgentError>>;
    fn rewind(&self, session_id: &str, turn_index: u32)
        -> BoxFuture<'_, Result<Session, AgentError>>;
    fn tag(&self, session_id: &str, tags: Vec<String>)
        -> BoxFuture<'_, Result<(), AgentError>>;
    fn rename(&self, session_id: &str, new_name: String)
        -> BoxFuture<'_, Result<(), AgentError>>;
}
```

Manages session persistence and lifecycle.

| Method | Description |
|--------|-------------|
| `list` | Return all session metadata, ordered by `updated_at` descending. |
| `resume` | Load the full `Session` by ID. |
| `save` | Create or update a session, refreshing `updated_at`. |
| `delete` | Remove a session and all associated data. |
| `fork` | Duplicate a session with a new ID. The copy shares history up to the fork point. `new_name` overrides the name or appends `" (fork)"`. |
| `rewind` | Truncate messages to `turn_index` (zero-based). Returns the modified session. |
| `tag` | Add tags. Duplicate tags are silently ignored. |
| `rename` | Update the human-readable session name. |

`InMemorySessionManager` (in `synwire_agent`) is an ephemeral implementation
suitable for testing. A persistent SQLite implementation is in `synwire-checkpoint`.

---

### `ModelProvider`

```rust
pub trait ModelProvider: Send + Sync {
    fn list_models(&self)
        -> BoxFuture<'_, Result<Vec<ModelInfo>, AgentError>>;
}
```

Implemented by LLM provider crates (`synwire-llm-openai`, `synwire-llm-ollama`).
Returns the set of models offered by the provider along with their capabilities
and context window sizes.

---

## Backend Traits — `synwire_core::vfs`

---

### `Vfs`

```rust
pub trait Vfs: Send + Sync {
    fn ls(&self, path: &str)
        -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>>;
    fn read(&self, path: &str)
        -> BoxFuture<'_, Result<FileContent, VfsError>>;
    fn write(&self, path: &str, content: &[u8])
        -> BoxFuture<'_, Result<WriteResult, VfsError>>;
    fn edit(&self, path: &str, old: &str, new: &str)
        -> BoxFuture<'_, Result<EditResult, VfsError>>;
    fn grep(&self, pattern: &str, opts: GrepOptions)
        -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>>;
    fn glob(&self, pattern: &str)
        -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>>;
    fn upload(&self, from: &str, to: &str)
        -> BoxFuture<'_, Result<TransferResult, VfsError>>;
    fn download(&self, from: &str, to: &str)
        -> BoxFuture<'_, Result<TransferResult, VfsError>>;
    fn pwd(&self)
        -> BoxFuture<'_, Result<String, VfsError>>;
    fn cd(&self, path: &str)
        -> BoxFuture<'_, Result<(), VfsError>>;
    fn rm(&self, path: &str)
        -> BoxFuture<'_, Result<(), VfsError>>;
    fn cp(&self, from: &str, to: &str)
        -> BoxFuture<'_, Result<TransferResult, VfsError>>;
    fn mv_file(&self, from: &str, to: &str)
        -> BoxFuture<'_, Result<TransferResult, VfsError>>;
    fn capabilities(&self) -> VfsCapabilities;
}
```

Unified protocol for agent filesystem and storage operations. Backends that do
not support an operation return `VfsError::Unsupported`.

| Method | Description |
|--------|-------------|
| `ls` | List directory contents. |
| `read` | Read file bytes and optional MIME type. |
| `write` | Write bytes; creates or overwrites. |
| `edit` | Replace the first occurrence of `old` with `new` (text files only). |
| `grep` | Ripgrep-style content search with `GrepOptions`. |
| `glob` | Find paths matching a glob pattern. |
| `upload` | Copy a local file (`from`) to the backend (`to`). |
| `download` | Copy a backend file (`from`) to a local path (`to`). |
| `pwd` | Return the current working directory. |
| `cd` | Change the current working directory. |
| `rm` | Remove a file or directory. |
| `cp` | Copy within the backend. |
| `mv_file` | Move or rename within the backend. |
| `capabilities` | Return the `VfsCapabilities` bitflags supported. |

Implementations in `synwire_agent`: `LocalProvider`, `GitBackend`,
`HttpBackend`, `ProcessManager`, `ArchiveManager`, `StoreProvider`,
`MemoryProvider`, `Shell`, `CompositeProvider`.

---

### `SandboxVfs`

```rust
pub trait SandboxVfs: Send + Sync {
    fn execute(&self, cmd: &str, args: &[String])
        -> BoxFuture<'_, Result<ExecuteResponse, VfsError>>;
    fn execute_pipeline(&self, stages: &[PipelineStage])
        -> BoxFuture<'_, Result<Vec<ExecuteResponse>, VfsError>>;
    fn id(&self) -> &str;
}
```

Separate from `Vfs` to make command-execution capability explicit.

| Method | Description |
|--------|-------------|
| `execute` | Run a single command with arguments. |
| `execute_pipeline` | Run a sequence of stages; each stage's stdout is piped into the next. |
| `id` | Sandbox identifier used for logging and audit. |

`BaseSandbox` is a type alias for `dyn SandboxVfs + Send + Sync`.

---

### `ApprovalCallback`

```rust
pub trait ApprovalCallback: Send + Sync {
    fn request(&self, req: ApprovalRequest)
        -> BoxFuture<'_, ApprovalDecision>;
}
```

Gate for risky operations. Called before any operation whose `RiskLevel` requires
approval under the active `PermissionMode`.

`AutoApproveCallback` always returns `ApprovalDecision::Allow`.
`AutoDenyCallback` always returns `ApprovalDecision::Deny`.
`ThresholdGate` auto-approves operations at or below a configured `RiskLevel`
and delegates higher-risk operations to an inner callback.

---

## MCP Traits — `synwire_core::mcp`

---

### `McpTransport`

```rust
pub trait McpTransport: Send + Sync {
    fn connect(&self)
        -> BoxFuture<'_, Result<(), AgentError>>;
    fn reconnect(&self)
        -> BoxFuture<'_, Result<(), AgentError>>;
    fn disconnect(&self)
        -> BoxFuture<'_, Result<(), AgentError>>;
    fn status(&self)
        -> BoxFuture<'_, McpServerStatus>;
    fn list_tools(&self)
        -> BoxFuture<'_, Result<Vec<McpToolDescriptor>, AgentError>>;
    fn call_tool(&self, tool_name: &str, arguments: Value)
        -> BoxFuture<'_, Result<Value, AgentError>>;
}
```

Low-level transport for communicating with an MCP server.

| Method | Description |
|--------|-------------|
| `connect` | Establish connection. For `StdioMcpTransport`, this spawns the subprocess. |
| `reconnect` | Re-establish after a drop without changing configuration. |
| `disconnect` | Clean shutdown. |
| `status` | Return a `McpServerStatus` snapshot including call counters and connection state. |
| `list_tools` | Return all tools advertised by the server. |
| `call_tool` | Invoke a tool by name with JSON arguments; returns the tool's JSON response. |

Implementations in `synwire_agent`: `StdioMcpTransport`, `HttpMcpTransport`,
`InProcessMcpTransport`. These are managed by `McpLifecycleManager`.

---

### `OnElicitation`

```rust
pub trait OnElicitation: Send + Sync {
    fn elicit(&self, request: ElicitationRequest)
        -> BoxFuture<'_, Result<ElicitationResult, AgentError>>;
}
```

Receives mid-call requests for additional user input from an MCP server
(credentials, confirmations, etc.) and returns a `ElicitationResult`.

`CancelAllElicitations` is the default implementation — it cancels every request
without prompting.

---

## Store Trait — `synwire_agent`

---

### `BaseStore`

```rust
pub trait BaseStore: Send + Sync {
    fn get(&self, namespace: &str, key: &str)
        -> Result<Option<Vec<u8>>, VfsError>;
    fn set(&self, namespace: &str, key: &str, value: Vec<u8>)
        -> Result<(), VfsError>;
    fn delete(&self, namespace: &str, key: &str)
        -> Result<(), VfsError>;
    fn list(&self, namespace: &str)
        -> Result<Vec<String>, VfsError>;
}
```

Synchronous namespaced key-value store. Note: unlike most Synwire traits,
`BaseStore` methods are synchronous (`Result`, not `BoxFuture`).

| Method | Description |
|--------|-------------|
| `get` | Return the value for `namespace/key`, or `None` if absent. |
| `set` | Write `value` to `namespace/key`. Creates or overwrites. |
| `delete` | Remove `namespace/key`. Returns `VfsError::NotFound` if absent. |
| `list` | Return all keys in `namespace` (without the namespace prefix). |

`InMemoryStore` wraps a `BTreeMap` behind a `RwLock`.
`StoreProvider` wraps a `BaseStore` and exposes it as `Vfs` with
paths of the form `/<namespace>/<key>`.

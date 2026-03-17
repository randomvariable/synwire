# Types Reference

All public structs, enums, and type aliases. Enums marked `#[non_exhaustive]`
cannot be exhaustively matched outside this crate — always include a `_` arm.

---

## Agent Builder — `synwire_core::agents::agent_node`

---

### `Agent<O>`

Builder for configuring and constructing a runnable agent. `O` is the optional
structured output type; use `()` for unstructured text.

```rust
pub struct Agent<O: Serialize + Send + Sync + 'static = ()> { /* private */ }
```

**Builder methods** (all `#[must_use]`, all return `Self`):

| Method | Description |
|--------|-------------|
| `new(name, model)` | Create a builder with name and primary model set. |
| `description(s)` | Human-readable description. |
| `model(s)` | Override the primary model identifier. |
| `fallback_model(s)` | Model used when the primary is rate-limited or unavailable. |
| `effort(EffortLevel)` | Reasoning effort hint. |
| `thinking(ThinkingConfig)` | Extended thinking / chain-of-thought configuration. |
| `tool(t)` | Add a tool available to the agent. |
| `allowed_tools(iter)` | Allowlist of tool names (only these tools may be called). |
| `exclude_tool(name)` | Remove a tool by name from the effective set. |
| `plugin(p)` | Attach a plugin. |
| `middleware(mw)` | Append a middleware to the stack. |
| `hooks(HookRegistry)` | Register lifecycle hooks. |
| `output_mode(OutputMode)` | Configure structured output extraction. |
| `output_schema(Value)` | JSON Schema for output validation. |
| `max_turns(u32)` | Maximum turns per run. |
| `max_budget(f64)` | Maximum cumulative cost in USD. |
| `system_prompt(SystemPromptConfig)` | Append to or replace the base system prompt. |
| `permission_mode(PermissionMode)` | Permission preset. |
| `permission_rule(PermissionRule)` | Add a declarative permission rule. |
| `sandbox(SandboxConfig)` | Sandbox configuration. |
| `env(key, value)` | Set an environment variable available to the agent. |
| `cwd(path)` | Set the working directory. |
| `debug()` | Enable verbose debug logging. |
| `debug_file(path)` | Write debug output to a file. |
| `mcp_server(name)` | Register an MCP server by name. |
| `before_agent(f)` | Callback invoked before each turn. |
| `after_agent(f)` | Callback invoked after each turn (success or failure). |
| `on_model_error(f)` | Callback invoked on model errors; returns `ModelErrorAction`. |

`Agent<O>` implements `AgentNode`.

---

### `RunContext`

Runtime context made available during agent execution.

```rust
pub struct RunContext {
    pub session_id: Option<String>,
    pub model: String,
    pub retry_count: u32,
    pub cumulative_cost_usd: f64,
    pub metadata: HashMap<String, Value>,
}
```

| Field | Description |
|-------|-------------|
| `session_id` | Active session ID, or `None` for stateless runs. |
| `model` | Model identifier resolved for this run. |
| `retry_count` | Number of retries for the current turn (0 = first attempt). |
| `cumulative_cost_usd` | Total cost accumulated in this session so far. |
| `metadata` | Arbitrary metadata attached at the call site. |

---

### `ModelErrorAction`

Recovery action returned by an `OnModelErrorCallback`. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Retry` | Retry the current request unchanged. |
| `Abort(String)` | Abort the run with the given message. |
| `SwitchModel(String)` | Switch to the specified model and retry. |

---

## Runner — `synwire_core::agents::runner`

---

### `Runner<O>`

Drives the agent execution loop. Stateless between runs.

```rust
pub struct Runner<O: Serialize + Send + Sync + 'static = ()> { /* private */ }
```

| Method | Description |
|--------|-------------|
| `new(agent)` | Create a runner wrapping the given `Agent<O>`. |
| `async set_model(model)` | Switch to a different model for subsequent turns without resetting conversation history. |
| `async stop_graceful()` | Signal a graceful stop; the runner finishes any in-flight tool call then emits `TurnComplete { reason: Stopped }`. |
| `async stop_force()` | Signal an immediate stop; emits `TurnComplete { reason: Aborted }`. |
| `async run(input, config) -> Result<Receiver<AgentEvent>, AgentError>` | Start a run. Events arrive on the returned `mpsc::Receiver`. The stream ends after a `TurnComplete` or `Error` event. |

---

### `RunnerConfig`

Configuration for a single runner execution.

```rust
pub struct RunnerConfig {
    pub model_override: Option<String>,
    pub session_id: Option<String>,
    pub max_retries: u32,          // default: 3
}
```

| Field | Description |
|-------|-------------|
| `model_override` | Override the agent's model for this specific run. |
| `session_id` | Resume an existing session, or `None` for a new session. |
| `max_retries` | Maximum retries per model error before falling back or aborting. |

---

### `StopKind`

```rust
pub enum StopKind {
    Graceful,
    Force,
}
```

| Variant | Description |
|---------|-------------|
| `Graceful` | Drain in-flight tool calls, then stop. |
| `Force` | Cancel immediately without draining. |

---

### `RunErrorAction`

Action taken by the runner when an error occurs. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Retry` | Retry the current request (up to `max_retries`). |
| `Continue` | Ignore this error and advance to the next turn. |
| `Abort(String)` | Abort the run immediately. |
| `SwitchModel(String)` | Switch to the given model and retry. |

---

## Directive System — `synwire_core::agents::directive`

---

### `Directive`

Typed effect description returned by agent nodes. Directives describe side
effects without executing them, enabling pure unit tests. `#[non_exhaustive]`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum Directive { ... }
```

| Variant | Fields | Description |
|---------|--------|-------------|
| `Emit` | `event: AgentEvent` | Emit an event to the event stream. |
| `SpawnAgent` | `name: String`, `config: Value` | Request spawning a child agent. |
| `StopChild` | `name: String` | Request stopping a child agent. |
| `Schedule` | `action: String`, `delay: Duration` | Schedule a delayed action. Delay is serialised via `humantime_serde`. |
| `RunInstruction` | `instruction: String`, `input: Value` | Ask the runtime to execute an instruction and route the result back. |
| `Cron` | `expression: String`, `action: String` | Schedule a recurring action. |
| `Stop` | `reason: Option<String>` | Request agent stop. |
| `SpawnTask` | `description: String`, `input: Value` | Spawn a background task. |
| `StopTask` | `task_id: String` | Cancel a background task by ID. |
| `Custom` | `payload: Box<dyn DirectivePayload>` | User-defined directive. Requires `#[typetag::serde]` on the payload type. |

---

### `DirectiveResult<S>`

Combines a state update with zero or more directives. `S` must implement
`synwire_core::State`.

```rust
pub struct DirectiveResult<S: State> {
    pub state: S,
    pub directives: Vec<Directive>,
}
```

**Constructors:**

| Method | Description |
|--------|-------------|
| `state_only(s)` | No directives. |
| `with_directive(s, d)` | One directive. |
| `with_directives(s, ds)` | Multiple directives. |
| `From<S>` | Converts state directly (equivalent to `state_only`). |

---

## Errors

---

### `AgentError`

Top-level error for agent operations. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Model(ModelError)` | LLM API error (`#[from]` conversion from `ModelError`). |
| `Tool(String)` | Tool execution failure. |
| `Strategy(String)` | Execution strategy error. |
| `Middleware(String)` | Middleware error. |
| `Directive(String)` | Directive execution error. |
| `Backend(String)` | Backend operation error. |
| `Session(String)` | Session management error. |
| `Panic(String)` | Caught panic with message payload. |
| `BudgetExceeded(f64)` | Cost exceeded the configured budget (USD). |

---

### `ModelError`

LLM API error with retryability metadata. `#[non_exhaustive]`

| Variant | Retryable | Description |
|---------|-----------|-------------|
| `Authentication(String)` | No | API key or credential failure. |
| `Billing(String)` | No | Quota or billing error. |
| `RateLimit(String)` | Yes | Rate limit exceeded. |
| `ServerError(String)` | Yes | Provider server error (5xx). |
| `InvalidRequest(String)` | No | Malformed request. |
| `MaxOutputTokens` | No | Response exceeded the output token limit. |

**Methods:**

| Method | Returns | Description |
|--------|---------|-------------|
| `is_retryable()` | `bool` | `true` for `RateLimit` and `ServerError` variants. |

---

### `StrategyError`

Execution strategy error. `#[non_exhaustive]`

| Variant | Fields | Description |
|---------|--------|-------------|
| `InvalidTransition` | `current_state`, `attempted_action`, `valid_actions` | Action not valid from the current FSM state. |
| `GuardRejected(String)` | — | All guard conditions rejected the transition. |
| `NoInitialState` | — | `FsmStrategyBuilder::build` called without setting an initial state. |
| `Execution(String)` | — | General execution failure (e.g. mutex poisoned). |

---

### `DirectiveError`

Error from `DirectiveExecutor`. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `ExecutionFailed(String)` | Directive could not be executed. |
| `Unsupported(String)` | Directive type not supported by this executor. |

---

### `FilterDecision`

Decision returned by `DirectiveFilter::decision`. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Pass` | Directive passes through (may be modified). |
| `Suppress` | Directive is silently dropped. |
| `Reject` | Directive is rejected with an error. |

---

### `FilterChain`

Ordered sequence of `DirectiveFilter` implementations.

| Method | Description |
|--------|-------------|
| `new()` | Create an empty chain. |
| `add(filter)` | Append a filter. |
| `apply(directive) -> Option<Directive>` | Apply all filters in order. Returns `None` if any filter suppresses. |

---

### `VfsError`

Backend operation error. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `NotFound(String)` | File or directory not found. |
| `PermissionDenied(String)` | Insufficient permissions. |
| `IsDirectory(String)` | Expected file, got directory. |
| `PathTraversal { attempted, root }` | Path traversal attempt blocked. |
| `ScopeViolation { path, scope }` | Path outside the allowed scope. |
| `ResourceLimit(String)` | Resource limit exceeded. |
| `Timeout(String)` | Operation timed out. |
| `OperationDenied(String)` | Denied by approval gate. |
| `Unsupported(String)` | Operation not supported by this backend. |
| `Io(io::Error)` | I/O error (`#[from]` conversion). |

---

## Streaming — `synwire_core::agents::streaming`

---

### `AgentEvent`

Streaming event produced during an agent run. `#[non_exhaustive]`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum AgentEvent { ... }
```

| Variant | Key fields | Description |
|---------|-----------|-------------|
| `TextDelta` | `content: String` | Streaming text chunk from the model. |
| `ToolCallStart` | `id: String`, `name: String` | Tool invocation started. |
| `ToolCallDelta` | `id: String`, `arguments_delta: String` | Streaming argument fragment. |
| `ToolCallEnd` | `id: String` | Tool invocation arguments complete. |
| `ToolResult` | `id: String`, `output: ToolOutput` | Tool execution result. |
| `ToolProgress` | `id: String`, `message: String`, `progress_pct: Option<f32>` | Progress report from a long-running tool. |
| `StateUpdate` | `patch: Value` | JSON patch to apply to agent state. |
| `DirectiveEmitted` | `directive: Value` | Agent emitted a directive (serialised). |
| `StatusUpdate` | `status: String`, `progress_pct: Option<f32>` | Human-readable status message. |
| `UsageUpdate` | `usage: Usage` | Token and cost counters for the current turn. |
| `RateLimitInfo` | `utilization_pct: f32`, `reset_at: i64`, `allowed: bool` | Rate limit information. |
| `TaskNotification` | `task_id: String`, `kind: TaskEventKind`, `payload: Value` | Background task lifecycle event. |
| `PromptSuggestion` | `suggestions: Vec<String>` | Model-suggested follow-up prompts. |
| `TurnComplete` | `reason: TerminationReason` | Turn ended. Always the last event unless `Error` occurs first. |
| `Error` | `message: String` | Fatal error; no further events follow. |

**Method:**

| Method | Returns | Description |
|--------|---------|-------------|
| `is_final_response()` | `bool` | `true` for `TurnComplete` and `Error`. |

---

### `AgentEventStream`

```rust
pub type AgentEventStream =
    Pin<Box<dyn Stream<Item = Result<AgentEvent, AgentError>> + Send>>;
```

Type alias for the stream returned by `AgentNode::run`.

---

### `TerminationReason`

Why a turn ended. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Complete` | Agent finished normally. |
| `MaxTurnsExceeded` | `max_turns` limit reached. |
| `BudgetExceeded` | `max_budget` limit reached. |
| `Stopped` | Graceful stop requested via `Runner::stop_graceful`. |
| `Aborted` | Force stop requested via `Runner::stop_force`. |
| `Error` | Terminated due to an unrecoverable error. |

---

### `TaskEventKind`

Lifecycle event for background tasks. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Started` | Task began executing. |
| `Progress` | Task reported intermediate progress. |
| `Completed` | Task finished successfully. |
| `Failed` | Task encountered an error. |

---

## Session — `synwire_core::agents::session`

---

### `Session`

Full snapshot of a conversation, persisted by `SessionManager`.

```rust
pub struct Session {
    pub metadata: SessionMetadata,
    pub messages: Vec<Value>,
    pub state: Value,
}
```

| Field | Description |
|-------|-------------|
| `metadata` | Identity and statistics. |
| `messages` | Conversation history as an array of JSON message objects. |
| `state` | Arbitrary agent state (plugin state, environment, etc.) serialised as JSON. |

---

### `SessionMetadata`

```rust
pub struct SessionMetadata {
    pub id: String,
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub agent_name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub turn_count: u32,
    pub total_tokens: u64,
}
```

| Field | Description |
|-------|-------------|
| `id` | Unique session identifier (UUID string). |
| `name` | Optional human-readable name. |
| `tags` | User-defined tags for filtering and search. |
| `agent_name` | Name of the agent this session belongs to. |
| `created_at` | Unix milliseconds at creation. |
| `updated_at` | Unix milliseconds at last save. |
| `turn_count` | Number of conversation turns recorded. |
| `total_tokens` | Cumulative token usage. |

---

## Model and Config — `synwire_core::agents`

---

### `ModelInfo`

```rust
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub capabilities: ModelCapabilities,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supported_effort_levels: Vec<EffortLevel>,
}
```

---

### `ModelCapabilities`

```rust
pub struct ModelCapabilities {
    pub tool_calling: bool,
    pub vision: bool,
    pub streaming: bool,
    pub structured_output: bool,
    pub effort_levels: bool,
}
```

---

### `EffortLevel`

Reasoning effort hint. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Low` | Minimal reasoning. |
| `Medium` | Moderate reasoning. |
| `High` | Deep reasoning (default). |
| `Max` | Maximum reasoning. |

---

### `ThinkingConfig`

Extended thinking / chain-of-thought configuration. `#[non_exhaustive]`

| Variant | Fields | Description |
|---------|--------|-------------|
| `Adaptive` | — | Model decides reasoning depth. |
| `Enabled` | `budget_tokens: u32` | Fixed token budget for reasoning. |
| `Disabled` | — | No reasoning. |

---

### `Usage`

Token usage and cost for a single turn.

```rust
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cost_usd: f64,
    pub context_utilization_pct: f32,
}
```

`context_utilization_pct` is in the range `0.0–1.0`.

---

### `OutputMode`

How the agent extracts structured output from the model response.
`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Tool` | Via tool call (most reliable). Default. |
| `Native` | Native JSON mode (provider must support it). |
| `Prompt` | Post-process raw text via prompt. |
| `Custom` | User-supplied extraction function. |

---

### `SystemPromptConfig`

`#[non_exhaustive]`

| Variant | Fields | Description |
|---------|--------|-------------|
| `Append` | `content: String` | Append to the base system prompt. |
| `Replace` | `content: String` | Replace the base system prompt entirely. |

---

### `PermissionMode`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Default` | Prompt for dangerous operations. Default. |
| `AcceptEdits` | Auto-approve file modifications. |
| `PlanOnly` | Read-only; no mutations allowed. |
| `BypassAll` | Auto-approve everything. |
| `DenyUnauthorized` | Deny unless a matching `PermissionRule` allows. |

---

### `PermissionBehavior`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Allow` | Allow the operation. |
| `Deny` | Deny the operation. |
| `Ask` | Prompt the user. |

---

### `PermissionRule`

Declarative permission rule.

```rust
pub struct PermissionRule {
    pub tool_pattern: String,
    pub behavior: PermissionBehavior,
}
```

`tool_pattern` is a glob matched against tool names.

---

### `SandboxConfig`

```rust
pub struct SandboxConfig {
    pub enabled: bool,
    pub network: Option<NetworkConfig>,
    pub filesystem: Option<FilesystemConfig>,
    pub allowed_commands: Option<Vec<String>>,
    pub denied_commands: Vec<String>,
}
```

---

### `NetworkConfig`

```rust
pub struct NetworkConfig {
    pub enabled: bool,
    pub allowed_domains: Option<Vec<String>>,
    pub denied_domains: Vec<String>,
}
```

`allowed_domains = None` means all domains are permitted.

---

### `FilesystemConfig`

```rust
pub struct FilesystemConfig {
    pub allowed_roots: Vec<String>,
    pub denied_paths: Vec<String>,
}
```

---

## Signals — `synwire_core::agents::signal`

---

### `Signal`

```rust
pub struct Signal {
    pub kind: SignalKind,
    pub payload: Value,
}
```

**Constructor:** `Signal::new(kind, payload)` — both fields are public but the
constructor is provided for convenience.

---

### `SignalKind`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Stop` | User requested stop. |
| `UserMessage` | User message received. |
| `ToolResult` | Tool invocation result available. |
| `Timer` | Timer or cron event. |
| `Custom(String)` | Application-defined signal. |

---

### `SignalRoute`

Maps a signal kind (with optional predicate) to an action.

```rust
pub struct SignalRoute {
    pub kind: SignalKind,
    pub predicate: Option<fn(&Signal) -> bool>,
    pub action: Action,
    pub priority: i32,
}
```

The predicate field uses a function pointer (not a closure) so `SignalRoute`
remains `Clone + Send + Sync`.

**Constructors:**

| Method | Description |
|--------|-------------|
| `new(kind, action, priority)` | Route without predicate. |
| `with_predicate(kind, predicate, action, priority)` | Route with a predicate. |

---

### `Action`

Action resulting from signal routing. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Continue` | Continue processing normally. |
| `GracefulStop` | Stop after draining in-flight work. |
| `ForceStop` | Stop immediately. |
| `Transition(String)` | Transition the FSM to the named state. |
| `Custom(String)` | Application-defined action identifier. |

---

### `ComposedRouter`

Three-tier signal router (strategy > agent > plugin). Within a tier, the route
with the highest `priority` value wins.

```rust
pub struct ComposedRouter { /* private */ }
```

| Constructor | Description |
|-------------|-------------|
| `new(strategy_routes, agent_routes, plugin_routes)` | Build a composed router from three priority tiers. |

Implements `SignalRouter`.

---

## Hooks — `synwire_core::agents::hooks`

---

### `HookRegistry`

Registry of lifecycle hooks with typed registration and per-hook timeout
enforcement. Hooks that exceed their timeout are skipped with a `warn!` log
and treated as `HookResult::Continue`.

**Registration methods:**

| Method | Hook point |
|--------|-----------|
| `on_pre_tool_use(matcher, f)` | Before a tool is called. |
| `on_post_tool_use(matcher, f)` | After a tool succeeds. |
| `on_post_tool_use_failure(matcher, f)` | After a tool fails. |
| `on_notification(matcher, f)` | On notification events. |
| `on_subagent_start(matcher, f)` | When a sub-agent starts. |
| `on_subagent_stop(matcher, f)` | When a sub-agent stops. |
| `on_pre_compact(matcher, f)` | Before conversation compaction. |
| `on_post_compact(matcher, f)` | After conversation compaction. |
| `on_session_start(matcher, f)` | When a session starts. |
| `on_session_end(matcher, f)` | When a session ends. |

Hook functions have signature `Fn(ContextType) -> BoxFuture<'static, HookResult>`.

---

### `HookMatcher`

Selects which events a hook applies to.

```rust
pub struct HookMatcher {
    pub tool_name_pattern: Option<String>,
    pub timeout: Duration,                  // default: 30 seconds
}
```

`tool_name_pattern` supports `*` as a wildcard. `None` matches all events.
`timeout` is enforced per-hook invocation.

---

### `HookResult`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Continue` | Proceed with normal execution. |
| `Abort(String)` | Abort the current operation. |

---

## Plugin System — `synwire_core::agents::plugin`

---

### `PluginStateMap`

Type-keyed map for isolated plugin state. Access is via `PluginStateKey`
implementations; plugins cannot read each other's state.

| Method | Description |
|--------|-------------|
| `new()` | Create an empty map. |
| `register::<P>(state) -> Result<PluginHandle<P>, &'static str>` | Register state for plugin `P`. Fails if `P` is already registered (returns `P::KEY` as `Err`). |
| `get::<P>() -> Option<&P::State>` | Immutable borrow. |
| `get_mut::<P>() -> Option<&mut P::State>` | Mutable borrow. |
| `insert::<P>(state)` | Insert or replace state unconditionally. |
| `serialize_all() -> Value` | Serialise all plugin state to a JSON object keyed by `P::KEY` strings. |

---

### `PluginHandle<P>`

Zero-sized proof token returned by `PluginStateMap::register`. Holding a
`PluginHandle<P>` proves that plugin `P` has been registered. Implements
`Copy + Clone + Debug`.

---

### `PluginInput`

Context passed to plugin lifecycle hooks.

```rust
pub struct PluginInput {
    pub turn: u32,
    pub message: Option<String>,
}
```

---

## Middleware — `synwire_core::agents::middleware`

---

### `MiddlewareInput`

```rust
pub struct MiddlewareInput {
    pub messages: Vec<Value>,
    pub context: Value,
}
```

---

### `MiddlewareResult`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Continue(MiddlewareInput)` | Pass (possibly modified) input to the next middleware. |
| `Terminate(String)` | Halt the chain immediately. |

---

### `MiddlewareStack`

Ordered collection of `Middleware` implementations.

| Method | Description |
|--------|-------------|
| `new()` | Create an empty stack. |
| `push(mw)` | Append a middleware. |
| `async run(input) -> Result<MiddlewareResult, AgentError>` | Execute all middleware in order. Stops at the first `Terminate`. |
| `system_prompt_additions() -> Vec<String>` | Collect additions from all middleware in order. |
| `tools() -> Vec<Box<dyn Tool>>` | Collect tools from all middleware in order. |

---

## Execution Strategies — `synwire_core::agents::execution_strategy`

---

### `FsmStateId`

Newtype wrapping `String` for FSM state identifiers. Implements
`From<&str> + From<String> + Hash + Eq`.

---

### `ActionId`

Newtype wrapping `String` for action identifiers. Implements
`From<&str> + From<String> + Hash + Eq`.

---

### `ClosureGuard`

Adapter wrapping `Fn(&Value) -> bool` as a `GuardCondition`.

```rust
pub struct ClosureGuard { /* private */ }
```

| Constructor | Description |
|-------------|-------------|
| `new(name, f)` | Create a named closure guard. |

---

## `synwire_agent` — Implementations

---

### `FsmStrategy`

FSM-constrained execution strategy. Actions must be valid transitions from the
current state; guard conditions further restrict which transitions fire.

| Method | Description |
|--------|-------------|
| `builder() -> FsmStrategyBuilder` | Return a builder. |
| `current_state() -> Result<FsmStateId, StrategyError>` | Return the current FSM state. |

Implements `ExecutionStrategy`.

---

### `FsmStrategyWithRoutes`

`FsmStrategy` bundled with associated signal routes (produced by `FsmStrategyBuilder::build`).

Fields:
- `strategy: FsmStrategy` — public for direct state inspection.

Implements `ExecutionStrategy`.

---

### `FsmStrategyBuilder`

Builder for `FsmStrategyWithRoutes`.

| Method | Description |
|--------|-------------|
| `state(id)` | Declare a state (documentation only; states are inferred from transitions). |
| `transition(from, action, to)` | Add an unconditional transition. |
| `transition_with_guard(from, action, to, guard, priority)` | Add a guarded transition. Higher `priority` is evaluated first. |
| `initial(state)` | Set the initial FSM state. |
| `route(SignalRoute)` | Add a signal route contributed by this strategy. |
| `build() -> Result<FsmStrategyWithRoutes, StrategyError>` | Build. Fails with `StrategyError::NoInitialState` if no initial state was declared. |

---

### `FsmTransition`

```rust
pub struct FsmTransition {
    pub target: FsmStateId,
    pub guard: Option<Box<dyn GuardCondition>>,
    pub priority: i32,
}
```

---

### `DirectStrategy`

Executes actions immediately without state constraints. Input is passed through
unchanged.

```rust
pub struct DirectStrategy;  // Clone + Default
```

`DirectStrategy::new()` is equivalent to `DirectStrategy::default()`.
Implements `ExecutionStrategy`.

---

### `InMemorySessionManager`

Ephemeral `SessionManager` backed by a `tokio::sync::RwLock<HashMap>`. All
data is lost when the process exits. See `synwire-checkpoint` for persistence.

| Constructor | Description |
|-------------|-------------|
| `new()` | Create an empty manager. |

---

### `InMemoryStore`

In-memory `BaseStore` backed by `RwLock<BTreeMap<String, Vec<u8>>>`.

| Constructor | Description |
|-------------|-------------|
| `new()` | Create an empty store. |

---

### `StoreProvider`

Wraps a `BaseStore` and exposes it as `Vfs`. Keys are paths of the
form `/<namespace>/<key>`. Supports only `READ`, `WRITE`, `RM`.

| Constructor | Description |
|-------------|-------------|
| `new(namespace, store)` | Create a backend scoped to `namespace`. |

---

### `CompositeProvider`

Routes `Vfs` operations to the backend whose prefix is the longest
segment-boundary match. `/store` matches `/store/foo` but not `/storefront`.

```rust
pub struct CompositeProvider { /* private */ }
```

| Constructor | Description |
|-------------|-------------|
| `new(mounts: Vec<Mount>)` | Sorts mounts by descending prefix length automatically. |

For `grep`: delegates to the first mount that advertises `VfsCapabilities::GREP`.
For `glob`: aggregates results from all mounts that advertise `VfsCapabilities::GLOB`.

---

### `Mount`

```rust
pub struct Mount {
    pub prefix: String,
    pub backend: Box<dyn Vfs>,
}
```

---

### `ThresholdGate`

`ApprovalCallback` that auto-approves operations at or below a `RiskLevel`
threshold and delegates higher-risk operations to an inner callback. Caches
`AllowAlways` decisions for subsequent calls to the same operation.

| Constructor | Description |
|-------------|-------------|
| `new(threshold, inner)` | Create a threshold gate. |

---

### `McpLifecycleManager`

Manages multiple named MCP server connections: connects on start, reconnects
on disconnect, and supports runtime enable/disable.

| Method | Description |
|--------|-------------|
| `new()` | Create an empty manager. |
| `async register(name, transport, reconnect_delay)` | Register a transport under `name`. |
| `async start_all()` | Connect all enabled servers. |
| `async stop_all()` | Disconnect all servers. |
| `async enable(name)` | Enable and connect a named server. |
| `async disable(name)` | Disable and disconnect a named server. |
| `async all_status() -> Vec<McpServerStatus>` | Current status of all managed servers. |
| `async list_tools(server_name)` | List tools from a named server. |
| `async call_tool(server_name, tool_name, arguments)` | Call a tool; reconnects if the server is disconnected. |
| `spawn_health_monitor(self: Arc<Self>, interval)` | Spawn a background task that polls and reconnects dropped servers. |

---

### Stdio / Http / InProcess MCP Transports

All three implement `McpTransport`.

| Type | Config variant | Description |
|------|---------------|-------------|
| `StdioMcpTransport` | `McpServerConfig::Stdio` | Spawns a subprocess; communicates over stdin/stdout with newline-delimited JSON-RPC. |
| `HttpMcpTransport` | `McpServerConfig::Http` | Communicates over HTTP; supports optional bearer token authentication. |
| `InProcessMcpTransport` | `McpServerConfig::InProcess` | In-process server backed by registered tool definitions. |

---

### `SummarisationMiddleware`

Detects when conversation history exceeds configured thresholds and marks
the context for summarisation. The actual LLM summarisation call is injected
by provider crates.

| Constructor | Description |
|-------------|-------------|
| `new(thresholds)` | Create with custom thresholds. |
| `default()` | `max_messages = 50`, `max_tokens = 80_000`, `max_context_utilisation = 0.8`. |

---

### `SummarisationThresholds`

```rust
pub struct SummarisationThresholds {
    pub max_messages: Option<usize>,
    pub max_tokens: Option<usize>,
    pub max_context_utilisation: Option<f32>,
}
```

Any threshold set to `None` is not checked. `max_context_utilisation` is in
the range `0.0–1.0`.

---

## Backend Types — `synwire_core::vfs::types`

---

### `VfsCapabilities`

Bitflags struct. Individual flags:

| Flag | Operation |
|------|-----------|
| `LS` | List directory |
| `READ` | Read files |
| `WRITE` | Write files |
| `EDIT` | Edit files |
| `GREP` | Search content |
| `GLOB` | Find files |
| `UPLOAD` | Upload files |
| `DOWNLOAD` | Download files |
| `PWD` | Get working directory |
| `CD` | Change working directory |
| `RM` | Remove files |
| `CP` | Copy files |
| `MV` | Move files |
| `EXEC` | Execute commands |

---

### `DirEntry`

```rust
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
}
```

---

### `FileContent`

```rust
pub struct FileContent {
    pub content: Vec<u8>,
    pub mime_type: Option<String>,
}
```

---

### `WriteResult`

```rust
pub struct WriteResult {
    pub path: String,
    pub bytes_written: u64,
}
```

---

### `EditResult`

```rust
pub struct EditResult {
    pub path: String,
    pub edits_applied: usize,
    pub content_after: Option<String>,
}
```

`edits_applied` is `0` when the old string was not found; `1` when the first
occurrence was replaced.

---

### `GrepMatch`

```rust
pub struct GrepMatch {
    pub file: String,
    pub line_number: usize,   // 1-indexed; 0 when line_numbers is false
    pub column: usize,        // 0-indexed byte offset of match start
    pub line_content: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
}
```

In `GrepOutputMode::Count` mode, `line_number` holds the match count and
`line_content` holds its string representation.

---

### `GlobEntry`

```rust
pub struct GlobEntry {
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}
```

---

### `TransferResult`

```rust
pub struct TransferResult {
    pub path: String,
    pub bytes_transferred: u64,
}
```

Used by `upload`, `download`, `cp`, and `mv_file`.

---

### `FileInfo`

```rust
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: Option<DateTime<Utc>>,
    pub permissions: Option<u32>,    // Unix mode bits
}
```

---

### `ExecuteResponse`

```rust
pub struct ExecuteResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}
```

---

### `ProcessInfo`

```rust
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub cpu_pct: Option<f32>,
    pub mem_bytes: Option<u64>,
    pub parent_pid: Option<u32>,
    pub state: String,
}
```

---

### `JobInfo`

Background job information.

```rust
pub struct JobInfo {
    pub id: String,
    pub pid: Option<u32>,
    pub command: String,
    pub status: String,
}
```

---

### `ArchiveEntry`

```rust
pub struct ArchiveEntry {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,             // uncompressed size
}
```

---

### `ArchiveInfo`

```rust
pub struct ArchiveInfo {
    pub entries: Vec<ArchiveEntry>,
    pub format: String,
    pub compressed_size: u64,
}
```

---

### `PipelineStage`

One stage in a multi-command pipeline.

```rust
pub struct PipelineStage {
    pub command: String,
    pub args: Vec<String>,
    pub stderr_to_stdout: bool,
    pub timeout_secs: Option<u64>,
}
```

Used by `SandboxVfs::execute_pipeline`.

---

## GrepOptions — `synwire_core::vfs::grep_options`

---

### `GrepOptions`

Ripgrep-style search configuration. All fields have zero-value defaults.

```rust
pub struct GrepOptions {
    pub path: Option<String>,
    pub after_context: u32,
    pub before_context: u32,
    pub context: Option<u32>,     // symmetric; overrides before/after if set
    pub case_insensitive: bool,
    pub glob: Option<String>,
    pub file_type: Option<String>,
    pub max_matches: Option<usize>,
    pub output_mode: GrepOutputMode,
    pub multiline: bool,
    pub line_numbers: bool,
    pub invert: bool,
    pub fixed_string: bool,
}
```

`path = None` searches from the current working directory.
`file_type` accepts ripgrep-style type names: `rust`, `python`, `js`,
`typescript`, `json`, `yaml`, `toml`, `markdown`, `go`, `sh`.

---

### `GrepOutputMode`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Content` | Return matching lines with context. Default. |
| `FilesWithMatches` | Return only file paths that contain a match. |
| `Count` | Return match counts per file (via `GrepMatch::line_number`). |

---

## Approval — `synwire_core::vfs::approval`

---

### `ApprovalRequest`

```rust
pub struct ApprovalRequest {
    pub operation: String,
    pub description: String,
    pub risk: RiskLevel,
    pub timeout_secs: Option<u64>,
    pub context: Value,
}
```

---

### `ApprovalDecision`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Allow` | Permit this invocation. |
| `Deny` | Deny this invocation. |
| `AllowAlways` | Permit this and all future invocations of the same operation. |
| `Abort` | Abort the entire agent run. |
| `AllowModified { modified_context }` | Permit with substituted context. |

---

### `RiskLevel`

Ordered (`PartialOrd`) risk classification. `#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `None` | No meaningful risk (read-only). |
| `Low` | Reversible writes. |
| `Medium` | File deletions, overwrites. |
| `High` | System changes, process spawning. |
| `Critical` | Irreversible or destructive. |

---

### AutoApprove / AutoDeny Callbacks

| Type | Behaviour |
|------|-----------|
| `AutoApproveCallback` | Always returns `ApprovalDecision::Allow`. |
| `AutoDenyCallback` | Always returns `ApprovalDecision::Deny`. |

Both implement `ApprovalCallback + Clone + Default + Debug`.

---

### `MemoryProvider`

Ephemeral in-memory implementation of `Vfs`. All data lives for
the lifetime of the backend instance. Suitable for agent scratchpads and test
fixtures.

Supports: `LS`, `READ`, `WRITE`, `EDIT`, `GREP`, `GLOB`, `PWD`, `CD`, `RM`,
`CP`, `MV`.

| Constructor | Description |
|-------------|-------------|
| `new()` | Create an empty backend with `/` as the working directory. |

---

## MCP Types — `synwire_core::mcp`

---

### `McpServerConfig`

Connection configuration for an MCP server. `#[non_exhaustive]`

| Variant | Key fields | Description |
|---------|-----------|-------------|
| `Stdio` | `command`, `args`, `env` | Launch a subprocess and communicate over stdin/stdout. |
| `Http` | `url`, `auth_token`, `timeout_secs` | Connect to an HTTP MCP server. |
| `Sse` | `url`, `auth_token`, `timeout_secs` | Connect via Server-Sent Events transport. |
| `InProcess` | `name` | In-process server backed by registered tool definitions. |

**Method:** `transport_kind() -> &'static str` — returns `"stdio"`, `"http"`,
`"sse"`, or `"in-process"`.

---

### `McpServerStatus`

```rust
pub struct McpServerStatus {
    pub name: String,
    pub state: McpConnectionState,
    pub calls_succeeded: u64,
    pub calls_failed: u64,
    pub enabled: bool,
}
```

---

### `McpToolDescriptor`

```rust
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,   // JSON Schema
}
```

---

### `McpConnectionState`

`#[non_exhaustive]`

| Variant | Description |
|---------|-------------|
| `Disconnected` | Not yet connected. Default. |
| `Connecting` | Connection attempt in progress. |
| `Connected` | Ready to accept tool calls. |
| `Reconnecting` | Reconnection in progress after a drop. |
| `Shutdown` | Server has been shut down. |

---

### `ElicitationRequest`

```rust
pub struct ElicitationRequest {
    pub request_id: String,
    pub message: String,
    pub response_schema: Value,   // JSON Schema for the expected response
    pub required: bool,
}
```

---

### `ElicitationResult`

`#[non_exhaustive]`

| Variant | Fields | Description |
|---------|--------|-------------|
| `Provided` | `request_id: String`, `value: Value` | User provided a valid response. |
| `Cancelled` | `request_id: String` | User cancelled without providing a response. |

---

### `CancelAllElicitations`

Default `OnElicitation` implementation. Cancels every request without prompting.
Implements `Default + Debug`.

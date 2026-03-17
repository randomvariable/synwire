# Research: Agent Core Runtime

**Feature**: 003-agent-core | **Date**: 2026-03-15

## R1: Directive System Design

### Decision: Extensible enum with `typetag`-serialised Custom variant

### Rationale

Agent nodes return `DirectiveResult<S>` combining updated state and zero or more typed directives. Directives are pure data describing side effects — they are never executed by the agent itself, enabling deterministic unit testing.

The enum approach with a `Custom(Box<dyn DirectivePayload>)` variant balances type safety for known variants with extensibility for user-defined directives:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum Directive {
    Emit { event: AgentEvent },
    SpawnAgent { name: String, config: serde_json::Value },
    StopChild { name: String },
    Schedule { action: String, delay: Duration },
    RunInstruction { instruction: String, input: serde_json::Value },
    Cron { expression: String, action: String },
    Stop { reason: Option<String> },
    #[serde(with = "custom_directive_serde")]
    Custom(Box<dyn DirectivePayload>),
}
```

### Alternatives Considered

1. **All-trait-object (`Vec<Box<dyn DirectivePayload>>`)**: Maximum flexibility but loses pattern matching, makes filtering harder, and requires `typetag` for every variant including built-in ones.
2. **Sealed enum only (no Custom)**: Simpler but prevents user extension without forking.
3. **Free monad / GAT-based effects**: Theoretically pure but far too complex for practical use; Rust's type system makes this pattern unwieldy.

### Key Design Details

- **Serialization**: Built-in variants use `#[serde(tag = "type")]` for clean JSON. Custom variants use `typetag` crate for heterogeneous serialization (registers concrete types at link time via inventory).
- **DirectivePayload trait**: `Send + Sync + Debug + Clone + typetag::serde::Serialize + typetag::serde::Deserialize`. The `typetag` macro handles registry.
- **DirectiveResult<S>**: `struct DirectiveResult<S: State> { pub state: S, pub directives: Vec<Directive> }`. State changes are immediate; directives are deferred.
- **DirectiveExecutor trait**: `fn execute_directive(&self, directive: &Directive) -> BoxFuture<'_, Result<Option<serde_json::Value>, DirectiveError>>`. Default `NoOpExecutor` records without executing (for testing). Returns optional value for `RunInstruction` results routed back.
- **DirectiveFilter trait**: `fn filter(&self, directive: Directive) -> Option<Directive>`. Returns `None` to suppress, `Some(modified)` to transform. Chain of filters applied before executor.
- **Integration with existing `State`**: `DirectiveResult<S>` is generic over the existing `State` trait from `synwire-orchestrator`. No changes to `State` trait needed.

---

## R2: Execution Strategy Design

### Decision: Runtime FSM with enum states, builder API, `Mutex<FsmStateId>` for current state

### Rationale

A runtime state machine (not type-state pattern) is required because:
1. The same agent logic must work under both Direct and FSM strategies without recompilation
2. Transition tables are configured at build time, not compile time
3. Type-state pattern propagates generics everywhere, incompatible with `dyn Trait` usage

### Alternatives Considered

1. **Type-state pattern (statig crate)**: Compile-time guarantees but incompatible with configurable/dynamic strategies. Agent code would differ between strategies.
2. **rust-fsm crate**: Macro-heavy, generates enums per state machine. Not suitable for runtime configuration.
3. **Petri nets**: More powerful than FSM but unnecessary complexity for agent workflows.

### Key Design Details

```rust
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

| Decision | Choice | Rationale |
|----------|--------|-----------|
| FSM encoding | Runtime (enum + transition table) | Configurable via builder; same agent code under both strategies |
| Guard conditions | `Arc<dyn GuardCondition>` trait object | Clone + Send + Sync + named for error messages |
| State identity | `FsmStateId` / `ActionId` newtypes over `String` | Type safety without generics overhead |
| Interior mutability | `Mutex<FsmStateId>` for current state | `Send + Sync` compliant; strategies are shared across tasks |
| Builder validation | At `build()` time, not compile time | Matches runtime-configured FSM approach |
| Snapshot serialization | `serde_json::Value` | Checkpoint-compatible with existing `State::to_value()` pattern |

- **DirectStrategy**: Executes immediately. `execute()` runs the action and returns. `tick()` returns `None` (no pending work). No state to track.
- **FsmStrategy**: Validates action against current state's allowed transitions. If valid, executes and transitions. If invalid, returns `InvalidTransition { current_state, attempted_action, valid_actions }`.
- **FsmTransition**: `{ from: FsmStateId, to: FsmStateId, action: ActionId, guard: Option<Arc<dyn GuardCondition>> }`.
- **Builder**: `FsmStrategy::builder().state("idle").state("processing").transition("idle", "process", "processing").guard("idle", "process", my_guard).build()`.

---

## R3: Plugin State Isolation

### Decision: TypeMap (anymap-style) with `PluginHandle<P>` tokens and const assertion for key collision

### Rationale

The TypeMap hybrid approach (Approach C from research) provides the best balance:
- Near-zero-cost access (~5ns for HashMap lookup + TypeId comparison, irrelevant vs LLM latency)
- Clean API: `plugin_state::<P>()` returns `&P::State`
- Structural isolation: node functions receive `NodeContext<'a, S, P>` with only their own plugin state in scope
- Forward-compatible: adding plugins doesn't change container types

### Alternatives Considered

1. **HList (frunk)**: True zero-cost but type complexity propagates everywhere, poor compile errors, incompatible with dynamic plugin composition.
2. **Plain HashMap<String, serde_json::Value>**: No type safety, runtime deserialization cost on every access.
3. **Separate fields per plugin (code generation)**: Maximum performance but requires proc macro for composition, not extensible.

### Key Design Details

```rust
pub trait PluginStateKey: 'static + Send + Sync {
    type State: Send + Sync + Default + Serialize + DeserializeOwned + Clone;
    const KEY: &'static str;
}

pub struct PluginHandle<P: PluginStateKey>(PhantomData<P>);

pub struct PluginStateMap { /* TypeId -> Box<dyn Any + Send + Sync> */ }
```

- **Key collision detection**: Const assertion via proc macro on `#[agent(plugins = [A, B, C])]`. Compares `KEY` strings at compile time using `const fn` string comparison. Falls back to runtime check at registration for non-macro usage.
- **Serialization for checkpointing**: Each entry stores a `fn(&dyn Any) -> serde_json::Value` alongside the value, captured at registration time when the concrete type is known.
- **No external dependencies**: The TypeMap is ~40 lines. Avoid `anymap2` to keep dependency count low (project already has many deps).

---

## R4: Backend Protocol Architecture

### Decision: Two-layer trait (dyn-safe `BackendOps` + ergonomic `BackendProtocol` extension), `Mutex<PathBuf>` for cwd, sorted vec for composite routing

### Rationale

RPITIT is available on stable Rust 1.85 but is not dyn-safe. Since `CompositeBackend` needs `Box<dyn BackendOps>`, we need a dyn-safe base trait with `Pin<Box<dyn Future>>` returns, plus an ergonomic extension trait with `async fn` for concrete usage.

### Alternatives Considered

1. **Single `async_trait` macro**: Adds proc macro dependency, hides the boxing, slightly worse error messages. The project already uses manual `BoxFuture` (see `Tool` trait and `RunnableCore`).
2. **Single trait with all `BoxFuture` returns**: Works but callers always write `.await` on boxed futures which is slightly less ergonomic. Since the existing codebase already uses this pattern (`Tool::invoke`, `RunnableCore::invoke`), **this is actually the most consistent choice**.
3. **Separate sync/async traits**: Doubles the API surface with no benefit.

### Revised Decision: Single trait with `BoxFuture` (matching existing codebase patterns)

Given that `synwire-core` already uses `BoxFuture` throughout (`Tool::invoke`, `RunnableCore::invoke`, `RunnableCore::batch`, `RunnableCore::stream`), the backend protocol should follow the same pattern for consistency:

```rust
pub trait BackendProtocol: Send + Sync {
    fn ls(&self, path: &str, opts: &LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, BackendError>>;
    fn read(&self, path: &str, opts: &ReadOptions) -> BoxFuture<'_, Result<FileContent, BackendError>>;
    fn write(&self, path: &str, content: &[u8], opts: &WriteOptions) -> BoxFuture<'_, Result<WriteResult, BackendError>>;
    fn edit(&self, path: &str, edits: &[EditOp]) -> BoxFuture<'_, Result<EditResult, BackendError>>;
    fn grep(&self, pattern: &str, opts: &GrepOptions) -> BoxFuture<'_, Result<Vec<GrepMatch>, BackendError>>;
    fn glob(&self, pattern: &str, opts: &GlobOptions) -> BoxFuture<'_, Result<Vec<GlobEntry>, BackendError>>;
    fn upload(&self, local: &str, remote: &str) -> BoxFuture<'_, Result<TransferResult, BackendError>>;
    fn download(&self, remote: &str, local: &str) -> BoxFuture<'_, Result<TransferResult, BackendError>>;
    fn pwd(&self) -> Result<String, BackendError>;
    fn cd(&self, path: &str) -> Result<(), BackendError>;
    fn rm(&self, path: &str, opts: &RmOptions) -> BoxFuture<'_, Result<(), BackendError>>;
    fn cp(&self, src: &str, dst: &str, opts: &CpOptions) -> BoxFuture<'_, Result<(), BackendError>>;
    fn mv_file(&self, src: &str, dst: &str) -> BoxFuture<'_, Result<(), BackendError>>;
    fn capabilities(&self) -> BackendCapabilities;
}
```

### Key Design Details

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Async trait approach | Single trait with `BoxFuture` | Matches existing `Tool`/`RunnableCore` pattern in codebase |
| Sync support | `BlockingBackend<B>` adapter using `Handle::block_on` | No trait duplication |
| Composite routing | Sorted `Vec<Mount>` by descending prefix length, segment-boundary matching | Simple, correct, fast for <50 mounts |
| Working directory | `Mutex<PathBuf>` per backend | Interior mutability with `&self`, low contention |
| Approval gates | `ApprovalGate` trait, async, composable with `ThresholdGate` | Supports human-in-the-loop |
| Capabilities | `bitflags` for `BackendCapabilities` | Zero-cost, composable |
| Path safety | `resolve()` canonicalizes then checks `starts_with(root)` | Reuses existing `security::path` module |
| Error type | Single `#[non_exhaustive]` `BackendError` with `thiserror` | Forward-compatible, structured |

- **GrepOptions**: Mirrors ripgrep flags: context lines (before/after/symmetric), case sensitivity, file type filter, max matches, line numbers, invert match, count mode, multiline, word boundary, fixed string. All fields default via `Default` trait.
- **SandboxBackendProtocol**: Extends `BackendProtocol` with `execute`, `execute_pipeline`, stream redirection, and `id` property. Inherits all file operations.
- **CompositeBackend**: Maintains own `Mutex<String>` for cwd. Routes by longest path prefix match on segment boundaries (`/store/data` matches `/store/data/file` but not `/store/database/x`).
- **pwd/cd are sync**: Only mutate/read internal `Mutex<PathBuf>`, no I/O needed.

---

## R5: Middleware Stack Design

### Decision: Ordered `Vec<Box<dyn Middleware>>` with `MiddlewareResult` enum for early termination

### Rationale

Follows the established pattern from web frameworks (tower, actix middleware). Each middleware wraps the next, enabling pre-processing of inputs and post-processing of outputs.

### Key Design Details

```rust
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

pub enum MiddlewareResult {
    Continue(AgentResult),
    Terminate(AgentResult),
}
```

- Each middleware in the stack can add tools, modify system prompts, transform state, or short-circuit execution.
- Middleware that expose backend operations as tools (FilesystemMiddleware, GitMiddleware, etc.) implement `tools()` to register their tool set.
- Stack order is configurable via the agent builder.

---

## R6: Signal Routing

### Decision: Three-tier priority with first-match-wins, `SignalRouter` trait

### Rationale

Matches the spec requirement. Strategy routes have highest priority (can gate signals based on execution state), agent routes are middle, plugin routes are lowest.

```rust
pub trait SignalRouter: Send + Sync {
    fn route(&self, signal: &Signal) -> Option<Action>;
}
```

- `ComposedRouter` holds `(Vec<SignalRoute>, Vec<SignalRoute>, Vec<SignalRoute>)` for strategy, agent, plugin tiers.
- Routing decisions logged at `tracing::debug!` level with tier and matched route.

---

## R7: Streaming Events

### Decision: `AgentEvent` enum with `is_final_response()` method, `turn_complete` signal

### Rationale

Follows the spec. Consumers need to distinguish partial streaming updates from final results.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum AgentEvent {
    TextDelta { content: String },
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, arguments_delta: String },
    ToolCallEnd { id: String },
    ToolResult { id: String, output: ToolOutput },
    StateUpdate { patch: serde_json::Value },
    DirectiveEmitted { directive: Directive },
    TurnComplete,
    Error { message: String },
}

impl AgentEvent {
    pub fn is_final_response(&self) -> bool {
        matches!(self, AgentEvent::TurnComplete | AgentEvent::Error { .. })
    }
}
```

---

## R8: New Dependencies Required

| Dependency | Purpose | Justification |
|-----------|---------|---------------|
| `typetag` | Heterogeneous directive serialization | Required for Custom directive variant round-trip |
| `bitflags` | Backend capabilities | Zero-cost capability introspection |
| `parking_lot` (optional) | Faster Mutex for cwd | Optional perf improvement, std Mutex is fine |

No other new dependencies needed. The project already has `serde`, `tokio`, `thiserror`, `futures-util`, `reqwest`, `tracing`, `regex` — all sufficient for the implementation.

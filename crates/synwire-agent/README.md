# synwire-agent

The agent runtime for Synwire. Concrete implementations of all `synwire-core` agent traits: `Runner`, execution strategies, backends, middleware, MCP transports, and session management.

## What this crate provides

- **`Runner`** — drives the agent turn loop: `AgentNode` + `ExecutionStrategy` + middleware + backend + session
- **`DirectStrategy`** — unconstrained execution; model decides everything
- **`FsmStrategy` / `FsmStrategyBuilder`** — finite state machine with guards, priorities, and named transitions
- **10 `BackendProtocol` implementations** — `StateBackend`, `FilesystemBackend`, `GitBackend`, `HttpBackend`, `LocalShellBackend`, `ProcessBackend`, `ArchiveBackend`, `StoreBackend`, `PipelineExecutor`, `CompositeBackend`
- **9 `Middleware` types** — `ToolInjection`, `SystemPrompt`, `RateLimit`, `AuditLog`, `Summarisation`, `ConversationHistory`, `Caching`, `Timeout`, `CircuitBreaker`
- **MCP transports** — `StdioMcpTransport`, `HttpMcpTransport`, `InProcessMcpTransport`, `McpLifecycleManager`
- **`InMemorySessionManager`** — in-process session storage
- **Approval gates** — `ThresholdGate`, `RiskLevel`, `ApprovalDecision`
- **Signals** — `Signal`, `SignalKind`, `Action`, `SignalRoute`, `ComposedRouter`
- **Permissions** — `PermissionMode`, `PermissionRule`, `PermissionBehavior`

## Quick start

```toml
[dependencies]
synwire-agent = "0.1"
synwire-core = "0.1"
tokio = { version = "1", features = ["full"] }
```

Run an agent with the direct strategy:

```rust,no_run
use synwire_agent::runner::Runner;
use synwire_agent::strategies::DirectStrategy;
use synwire_agent::backends::StateBackend;

// Assume `my_agent` implements `AgentNode`
// let runner = Runner::builder()
//     .agent(my_agent)
//     .strategy(DirectStrategy::new())
//     .backend(StateBackend::new())
//     .build()?;
//
// let mut events = runner.run("Hello, agent!".to_string(), Default::default()).await?;
// while let Some(event) = events.next().await {
//     println!("{:?}", event?);
// }
```

FSM strategy with two states and a guard:

```rust,no_run
use synwire_agent::strategies::{FsmStrategyBuilder, ClosureGuard};

// let strategy = FsmStrategyBuilder::new()
//     .add_state("idle")
//     .add_state("working")
//     .set_initial_state("idle")
//     .add_transition("idle", "working", ClosureGuard::new(|directive| {
//         matches!(directive, Directive::RunInstruction { .. })
//     }))
//     .add_transition("working", "idle", ClosureGuard::always())
//     .build()?;
```

Middleware stack:

```rust,no_run
use synwire_agent::middleware::{SystemPromptMiddleware, AuditLogMiddleware};

// let runner = Runner::builder()
//     .middleware(SystemPromptMiddleware::new("You are a helpful assistant."))
//     .middleware(AuditLogMiddleware::new(std::io::stderr()))
//     .build()?;
```

## Backend selection

| Backend | Scope | Use when |
|---|---|---|
| `StateBackend` | Ephemeral in-memory | Sandboxed agents, tests |
| `FilesystemBackend` | Scoped to root path | Reading/writing files safely |
| `GitBackend` | Within a git repo | Version-controlled operations |
| `HttpBackend` | External HTTP | Calling APIs |
| `LocalShellBackend` | Sandboxed working dir | Shell commands with scope control |
| `ProcessBackend` | Any | Spawning background jobs |
| `ArchiveBackend` | Scoped root | tar/zip read and write |
| `StoreBackend` | Namespaced K-V | Persistent agent state |
| `CompositeBackend` | Mount table | Different backends by path prefix |

## Documentation

- [Your First Agent](https://randomvariable.github.io/synwire/tutorials/01-first-agent.html)
- [Agent Runtime Explanation](https://randomvariable.github.io/synwire/explanation/synwire-agent.html)
- [When to use StateGraph vs FsmStrategy](https://randomvariable.github.io/synwire/explanation/graph-vs-agent.html)
- [Full API docs](https://docs.rs/synwire-agent)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

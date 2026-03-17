# synwire-agent: The Agent Runtime

`synwire-agent` is the concrete implementation layer for the Synwire agent runtime. It depends only on `synwire-core` and provides everything you need to build and run an agentic application: the `Runner`, execution strategies, backends, middleware, MCP transports, session management, and permissions.

> **Background**: [Agent Components](https://www.promptingguide.ai/agents/components) — the Prompt Engineering Guide covers memory, tools, and planning as the three components of an agent. `synwire-agent` implements all three: session management (memory), `Vfs` (tools), and `ExecutionStrategy` (planning).

## The `Runner`: entry point

`Runner` drives the agent turn loop. It ties together an `AgentNode` (the decision logic), an `ExecutionStrategy` (the turn-sequencing logic), a middleware stack, a backend, and a session.

```rust,no_run
use synwire_agent::runner::Runner;
use synwire_agent::strategies::DirectStrategy;
use synwire_agent::vfs::MemoryProvider;
use futures_util::StreamExt;

// Assume my_agent implements AgentNode
// let runner = Runner::builder()
//     .agent(my_agent)
//     .strategy(DirectStrategy::new())
//     .backend(MemoryProvider::new())
//     .build()?;
//
// let mut events = runner.run("Hello!".to_string(), Default::default()).await?;
// while let Some(event) = events.next().await {
//     match event? {
//         AgentEvent::Text { content } => print!("{content}"),
//         AgentEvent::Done { usage } => println!("\nTokens: {:?}", usage),
//         _ => {}
//     }
// }
```

## Execution strategies

Two built-in strategies:

### `DirectStrategy`

The model decides everything. No state machine constrains which directives the agent may emit. Use for open-ended assistants where you want maximum flexibility.

```rust,no_run
use synwire_agent::strategies::DirectStrategy;

let strategy = DirectStrategy::new();
```

### `FsmStrategy`

A finite state machine controls the turn sequence. Named states and typed guard conditions — closures over `Directive` values — determine which transitions fire.

```rust,no_run
use synwire_agent::strategies::{FsmStrategyBuilder, ClosureGuard};
use synwire_core::agents::directive::Directive;

let strategy = FsmStrategyBuilder::new()
    .add_state("idle")
    .add_state("executing")
    .set_initial_state("idle")
    .add_transition(
        "idle",
        "executing",
        ClosureGuard::new(|d| matches!(d, Directive::RunInstruction { .. })),
    )
    .add_transition("executing", "idle", ClosureGuard::always())
    .build()?;
```

For the design rationale, see [FSM Strategy Design](./agent-core-fsm-strategy-design.md). For when to choose `FsmStrategy` vs `StateGraph`, see [StateGraph vs FsmStrategy](./graph-vs-agent.md).

## Backends

All backends implement `Vfs`. The backend you choose determines the scope and risk of the agent's operations:

```rust,no_run
use synwire_agent::vfs::{
    MemoryProvider,           // ephemeral in-memory
    LocalProvider,      // scoped to a root path
    GitBackend,             // within a git repo
    HttpBackend,            // external HTTP calls
    Shell,      // sandboxed shell execution
};

// Ephemeral backend — safe for testing and sandboxed agents
let safe = MemoryProvider::new();

// Filesystem backend — path traversal is blocked by normalize_path()
let fs = LocalProvider::new("/workspace");

// Git backend — all operations stay within the repo
let git = GitBackend::new("/workspace/.git");
```

| Backend | Scope | Risk level |
|---|---|---|
| `MemoryProvider` | None (ephemeral) | None |
| `LocalProvider` | Rooted path | Low (scoped) |
| `GitBackend` | Git repo boundary | Low (version-controlled) |
| `HttpBackend` | External network | Medium |
| `Shell` | Sandboxed working dir | Medium |
| `ProcessManager` | Any process | High |
| `CompositeProvider` | Mount table | Depends on mounts |

## Middleware

Middleware runs before each turn in declaration order. Use middleware for: injecting tools, adding system prompts, rate limiting, or audit logging.

```rust,no_run
use synwire_agent::middleware::{
    SystemPromptMiddleware,
    ToolInjectionMiddleware,
    AuditLogMiddleware,
    RateLimitMiddleware,
};

// let runner = Runner::builder()
//     .middleware(SystemPromptMiddleware::new("You are a helpful assistant."))
//     .middleware(ToolInjectionMiddleware::new(vec![my_tool]))
//     .middleware(RateLimitMiddleware::tokens_per_minute(60_000))
//     .middleware(AuditLogMiddleware::new(std::io::stderr()))
//     .build()?;
```

Middleware **cannot** do post-processing — there is no reverse pass. For post-turn processing, use a `Plugin` with `after_run` lifecycle hooks.

## Permissions and approval gates

Configure what the agent is allowed to do:

```rust,no_run
use synwire_agent::permissions::{PermissionMode, PermissionRule, PermissionBehavior};
use synwire_agent::gates::{ThresholdGate, RiskLevel};

// Preset: block all tool calls by default
// let mode = PermissionMode::Restricted;

// Custom: require approval for shell commands
// let mode = PermissionMode::Custom(vec![
//     PermissionRule {
//         tool_pattern: "shell_*".to_string(),
//         behavior: PermissionBehavior::Ask,
//     },
// ]);

// Approval gate: trigger human review for HIGH+ risk directives
// let gate = ThresholdGate::new(RiskLevel::High, |req| {
//     println!("Approve directive {:?}? (y/n)", req.directive);
//     // read stdin, return ApprovalDecision::Allow or Deny
//     ApprovalDecision::Allow
// });
```

## MCP integration

Connect external tool servers via the Model Context Protocol:

```rust,no_run
use synwire_agent::mcp::{StdioMcpTransport, McpLifecycleManager};

// Spawn a subprocess MCP server and manage its lifecycle:
// let transport = StdioMcpTransport::new("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]);
// let manager = McpLifecycleManager::new(transport);
// manager.connect().await?;
// let tools = manager.list_tools().await?;
```

## Session management

```rust,no_run
use synwire_agent::sessions::InMemorySessionManager;
use synwire_core::agents::session::SessionManager;

let mgr = InMemorySessionManager::new();
let session = mgr.create("my-agent").await?;
// session.id, session.metadata.created_at, session.messages
```

## See also

- [Your First Agent](../tutorials/01-first-agent.md)
- [Execution Strategies Tutorial](../tutorials/03-execution-strategies.md)
- [StateGraph vs FsmStrategy](./graph-vs-agent.md)
- [Middleware Execution Model](./agent-core-middleware-execution-model.md)
- [Plugin State Isolation](./agent-core-plugin-state-isolation.md)
- [Three-Tier Signal Routing](./agent-core-three-tier-signal-routing.md)

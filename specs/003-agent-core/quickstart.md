# Quickstart: Agent Core Runtime

**Feature**: 003-agent-core | **Date**: 2026-03-15

## 1. Simple Agent with Directive Testing (5 lines)

```rust
use synwire::agent::prelude::*;

let agent = Agent::builder()
    .name("my-agent")
    .model(my_model)
    .tools(vec![search_tool, calc_tool])
    .build()?;

let result = agent.run("What is 2+2?").await?;
```

## 2. Pure Directive Testing (No Side Effects)

```rust
use synwire_core::agents::{Directive, DirectiveResult};

// Agent node is a pure function returning state + directives
fn my_node(state: MyState) -> DirectiveResult<MyState> {
    DirectiveResult {
        state: MyState { count: state.count + 1, ..state },
        directives: vec![
            Directive::Emit { event: AgentEvent::TextDelta { content: "done".into() } },
            Directive::SpawnAgent { name: "helper".into(), config: json!({}) },
        ],
    }
}

// Test: verify directives without executing any side effects
#[test]
fn test_my_node_returns_spawn_directive() {
    let result = my_node(MyState::default());
    assert_eq!(result.state.count, 1);
    assert!(matches!(&result.directives[1], Directive::SpawnAgent { name, .. } if name == "helper"));
    // No subprocess was spawned. No filesystem was touched. Pure data.
}
```

## 3. FSM Execution Strategy

```rust
use synwire_agent::strategies::FsmStrategy;

let strategy = FsmStrategy::builder()
    .state("idle")
    .state("processing")
    .state("done")
    .initial("idle")
    .transition("idle", "process", "processing")
    .transition("processing", "complete", "done")
    .guard("idle", "process", |ctx| ctx.has_input())
    .build()?;

let agent = Agent::builder()
    .name("stateful-agent")
    .model(my_model)
    .strategy(strategy)
    .build()?;

// Valid transition: idle -> processing
agent.execute("process", json!({"input": "data"})).await?;

// Invalid transition: processing -> processing (not defined)
let err = agent.execute("process", json!({})).await.unwrap_err();
assert!(matches!(err, StrategyError::InvalidTransition { .. }));
```

## 4. Plugin with Isolated State

```rust
use synwire_core::agents::plugin::PluginStateKey;

struct CachePlugin;
impl PluginStateKey for CachePlugin {
    type State = CacheState;
    const KEY: &'static str = "cache";
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct CacheState {
    hits: u32,
    entries: HashMap<String, String>,
}

let agent = Agent::builder()
    .name("cached-agent")
    .model(my_model)
    .plugin::<CachePlugin>()
    .build()?;
```

## 5. Backend with Approval Gates

```rust
use synwire_agent::backends::{FilesystemBackend, ThresholdGate};
use synwire_core::backends::{RiskLevel, ApprovalGate};

let gate = ThresholdGate::new(RiskLevel::Moderate, Some(my_interactive_gate));
let backend = FilesystemBackend::new("/workspace")
    .with_approval(gate);

// Read operations auto-approved (Safe level)
let content = backend.read("src/main.rs", &Default::default()).await?;

// Write operations auto-approved (Moderate level, within threshold)
backend.write("output.txt", b"result", &Default::default()).await?;

// Delete operations require interactive approval (Dangerous level)
backend.rm("important.db", &Default::default()).await?; // -> prompts user
```

## 6. Middleware Stack

```rust
use synwire_agent::middleware::*;

let agent = Agent::builder()
    .name("full-agent")
    .model(my_model)
    .middleware(FilesystemMiddleware::new(backend))
    .middleware(GitMiddleware::new(git_backend))
    .middleware(SummarisationMiddleware::new(100)) // summarise after 100 messages
    .middleware(PromptCachingMiddleware::new())
    .build()?;
```

## 7. Streaming Events

```rust
use futures_util::StreamExt;

let mut stream = agent.run_stream("Explain Rust ownership").await?;

while let Some(event) = stream.next().await {
    match event? {
        AgentEvent::TextDelta { content } => print!("{content}"),
        AgentEvent::TurnComplete => println!("\n--- Done ---"),
        _ => {}
    }
}
```

## 8. Three-Tier Signal Routing

```rust
// Strategy-level route (highest priority): reject input during processing
let strategy = FsmStrategy::builder()
    .state("processing")
    .route(SignalRoute {
        signal_kind: SignalKind::UserMessage,
        predicate: None,
        action: Action::Reject("Agent is processing, please wait".into()),
        priority: RoutePriority::Strategy,
    })
    .build()?;

// Agent-level route (middle priority)
let agent = Agent::builder()
    .route(SignalRoute {
        signal_kind: SignalKind::Interrupt,
        predicate: None,
        action: Action::Stop,
        priority: RoutePriority::Agent,
    })
    .build()?;
```

## Crate Map

| Crate | Contains |
|-------|----------|
| `synwire-core` | Traits: `DirectivePayload`, `ExecutionStrategy`, `Plugin`, `BackendProtocol`, `Middleware`, `AgentNode`, `SignalRouter` |
| `synwire-agent` | Implementations: `DirectStrategy`, `FsmStrategy`, `FilesystemBackend`, `GitBackend`, `HttpBackend`, all middleware |
| `synwire` | Re-exports: `synwire::agent::prelude::*` for convenience |
| `synwire-test-utils` | `NoOpExecutor`, backend conformance suite, proptest strategies |

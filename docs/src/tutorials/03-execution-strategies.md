# Controlling Agent Behaviour with Execution Strategies

**Time**: ~30 minutes
**Prerequisites**: Completed `02-pure-directive-testing.md`, basic understanding of
finite-state machines

An execution strategy constrains how an agent orchestrates actions. By default agents use
`DirectStrategy`, which accepts any action immediately. When you need to enforce an ordered
workflow — for example, an agent that must authenticate before it can process data — you
use `FsmStrategy` to model the allowed transitions as a state machine.

This tutorial covers both strategies and shows how to add guard conditions that inspect
input before allowing a transition.

---

## What you are building

1. A `DirectStrategy` agent that accepts any action.
2. An `FsmStrategy` for a simple three-state workflow: `idle → running → done`.
3. A guard that rejects a transition based on input content.
4. Snapshot serialisation to inspect FSM state.

---

## Step 1: Add dependencies

```toml
[dependencies]
synwire-core = { path = "../../crates/synwire-core" }
synwire-agent = { path = "../../crates/synwire-agent" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

---

## Step 2: DirectStrategy — immediate execution

`DirectStrategy` is the simplest implementation of `ExecutionStrategy`. It accepts any
action and passes the input through unchanged as its output. There is no state to set up
and no builder step — just construct and call:

```rust
use synwire_agent::strategies::DirectStrategy;
use synwire_core::agents::execution_strategy::ExecutionStrategy;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let strategy = DirectStrategy::new();

    // Execute any action — DirectStrategy never rejects.
    let output = strategy
        .execute("generate_text", json!({ "prompt": "hello" }))
        .await?;

    println!("output: {output}");

    // Snapshot tells you the strategy type.
    let snap = strategy.snapshot()?;
    let snap_value = snap.to_value()?;
    println!("snapshot: {snap_value}");  // {"type":"direct"}

    Ok(())
}
```

`DirectStrategy` is appropriate when:
- The agent orchestrates LLM calls without a defined state machine.
- You want no constraints on action ordering.
- You are prototyping and will add an FSM later.

---

## Step 3: FsmStrategy — state-constrained execution

`FsmStrategy` models the allowed actions as a directed graph. Each node is a _state_ and
each edge is an _(action, target-state)_ pair. The FSM rejects any action not defined for
the current state.

Build the FSM with `FsmStrategy::builder()`:

```rust
use synwire_agent::strategies::{FsmStrategy, FsmStrategyWithRoutes};
use synwire_core::agents::execution_strategy::{ExecutionStrategy, StrategyError};
use serde_json::json;

fn build_workflow_fsm() -> Result<FsmStrategyWithRoutes, StrategyError> {
    FsmStrategy::builder()
        // Declare states for readability (optional — states are inferred from transitions).
        .state("idle")
        .state("running")
        .state("done")
        // Set the state the FSM starts in.
        .initial("idle")
        // Define allowed transitions: (from, action, to).
        .transition("idle", "start", "running")
        .transition("running", "finish", "done")
        .build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fsm = build_workflow_fsm()?;

    // Valid transition: idle --start--> running.
    fsm.execute("start", json!({})).await?;
    println!("state: {:?}", fsm.strategy.current_state()?);  // running

    // Valid transition: running --finish--> done.
    fsm.execute("finish", json!({})).await?;
    println!("state: {:?}", fsm.strategy.current_state()?);  // done

    Ok(())
}
```

`FsmStrategy::builder().build()` returns `FsmStrategyWithRoutes` (not `FsmStrategy`
directly). `FsmStrategyWithRoutes` implements `ExecutionStrategy` and bundles any signal
routes defined on the builder. The inner `FsmStrategy` is available as the public
`.strategy` field when you need to call `current_state()`.

---

## Step 4: Handle invalid transitions

When you call `execute` with an action that has no transition from the current state, you
receive `StrategyError::InvalidTransition`:

```rust
#[tokio::test]
async fn test_invalid_transition() {
    let fsm = FsmStrategy::builder()
        .initial("idle")
        .state("idle")
        .state("running")
        .transition("idle", "start", "running")
        .build()
        .expect("valid FSM");

    // "finish" is not a valid action from "idle".
    let err = fsm
        .execute("finish", serde_json::json!({}))
        .await
        .expect_err("should reject unknown action");

    match err {
        StrategyError::InvalidTransition {
            current_state,
            attempted_action,
            valid_actions,
        } => {
            assert_eq!(current_state, "idle");
            assert_eq!(attempted_action, "finish");
            // valid_actions lists what IS allowed from the current state.
            assert!(valid_actions.contains(&"start".to_string()));
        }
        other => panic!("unexpected error: {other}"),
    }
}
```

The error message from `Display` is also human-readable:

```
Invalid transition from idle via finish. Valid actions: ["start"]
```

Note that `StrategyError` is `#[non_exhaustive]`; always include a catch-all arm in
`match` blocks.

---

## Step 5: Add a ClosureGuard

Guards let you inspect the action input before committing to a transition. Use
`transition_with_guard` and provide a `ClosureGuard`:

```rust
use synwire_agent::strategies::FsmStrategy;
use synwire_core::agents::execution_strategy::{ClosureGuard, ExecutionStrategy, StrategyError};
use serde_json::json;

#[tokio::test]
async fn test_guard_on_transition() {
    // Guard: only allow "start" when the input contains a non-empty "task" field.
    let has_task = ClosureGuard::new("requires-task", |input| {
        input
            .get("task")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
    });

    let fsm = FsmStrategy::builder()
        .initial("idle")
        .transition_with_guard("idle", "start", "running", has_task, 0)
        .build()
        .expect("valid FSM");

    // Input without "task" — guard rejects.
    let err = fsm
        .execute("start", json!({}))
        .await
        .expect_err("guard should reject");
    assert!(matches!(err, StrategyError::GuardRejected(_)));

    // Input with "task" — guard passes, transition succeeds.
    fsm.execute("start", json!({ "task": "summarise" }))
        .await
        .expect("guard should pass");
    assert_eq!(
        fsm.strategy.current_state().expect("state").0,
        "running"
    );
}
```

`ClosureGuard::new(name, f)` wraps any `Fn(&Value) -> bool` closure. The `name` string
appears in `StrategyError::GuardRejected` messages to help diagnose failures.

---

## Step 6: Priority ordering with multiple guards

When multiple transitions share the same `(from, action)` key, the FSM evaluates them in
descending priority order and accepts the first one whose guard passes:

```rust
use synwire_agent::strategies::FsmStrategy;
use synwire_core::agents::execution_strategy::{ClosureGuard, ExecutionStrategy};
use serde_json::json;

#[tokio::test]
async fn test_priority_ordering() {
    // Priority 10: premium path — only when input has "premium": true.
    let premium_guard = ClosureGuard::new("premium", |v| {
        v.get("premium").and_then(|p| p.as_bool()).unwrap_or(false)
    });

    // Priority 5: standard path — always passes.
    let standard_guard = ClosureGuard::new("always", |_| true);

    let fsm = FsmStrategy::builder()
        .initial("idle")
        .transition_with_guard("idle", "start", "premium-queue", premium_guard, 10)
        .transition_with_guard("idle", "start", "standard-queue", standard_guard, 5)
        .build()
        .expect("valid FSM");

    // Non-premium input: premium guard (priority 10) fails, standard guard (priority 5) passes.
    fsm.execute("start", json!({ "premium": false }))
        .await
        .expect("standard path");
    assert_eq!(
        fsm.strategy.current_state().expect("state").0,
        "standard-queue"
    );
}
```

The `priority` parameter is an `i32`. Higher values are evaluated first.

---

## Step 7: Snapshot the FSM state

`ExecutionStrategy::snapshot()` captures the current state of the strategy as a
`Box<dyn StrategySnapshot>`. Call `to_value()` to serialise it:

```rust
use synwire_agent::strategies::FsmStrategy;
use synwire_core::agents::execution_strategy::ExecutionStrategy;
use serde_json::json;

#[tokio::test]
async fn test_fsm_snapshot() {
    let fsm = FsmStrategy::builder()
        .initial("idle")
        .transition("idle", "start", "running")
        .build()
        .expect("valid FSM");

    // Snapshot before transition.
    let before = fsm.snapshot().expect("snapshot").to_value().expect("to_value");
    assert_eq!(before["type"], "fsm");
    assert_eq!(before["current_state"], "idle");

    fsm.execute("start", json!({})).await.expect("transition");

    // Snapshot after transition.
    let after = fsm.snapshot().expect("snapshot").to_value().expect("to_value");
    assert_eq!(after["current_state"], "running");
}
```

The snapshot for `DirectStrategy` is simpler:

```rust
use synwire_agent::strategies::DirectStrategy;
use synwire_core::agents::execution_strategy::ExecutionStrategy;

let snap = DirectStrategy::new()
    .snapshot()
    .expect("snapshot")
    .to_value()
    .expect("to_value");
assert_eq!(snap, serde_json::json!({"type": "direct"}));
```

Snapshots are useful for persisting workflow state to a checkpoint store so a long-running
agent can resume after a restart.

---

## Step 8: Signal routes

`FsmStrategyBuilder::route` attaches `SignalRoute` values to the built strategy. Signal
routes tell the agent runtime how to map incoming external signals to FSM actions. They are
declared at build time and queried via `ExecutionStrategy::signal_routes()`:

```rust
use synwire_agent::strategies::FsmStrategy;
use synwire_core::agents::signal::SignalRoute;
use synwire_core::agents::execution_strategy::ExecutionStrategy;

let fsm = FsmStrategy::builder()
    .initial("idle")
    .transition("idle", "start", "running")
    .route(SignalRoute {
        signal: "user.message".to_string(),
        action: "start".to_string(),
    })
    .build()
    .expect("valid FSM");

let routes = fsm.signal_routes();
assert_eq!(routes.len(), 1);
assert_eq!(routes[0].signal, "user.message");
assert_eq!(routes[0].action, "start");
```

See `../explanation/signal_routing.md` for how the runtime dispatches signals.

---

## Summary

| Strategy | When to use |
|---|---|
| `DirectStrategy` | No ordering constraints; any action is valid |
| `FsmStrategy` | Ordered workflow; reject invalid action sequences |
| `ClosureGuard` | Runtime inspection of action input before committing |

---

## Next steps

- **Plugin state**: Continue with `04-plugin-state-isolation.md` to learn how plugins
  attach isolated state slices to an agent.
- **Persisting snapshots**: See `../how-to/checkpointing.md` for storing FSM snapshots in
  the SQLite checkpoint backend.
- **Deep dive**: See `../explanation/execution_strategies.md` for the full lifecycle of a
  strategy within the runner loop.
## See also

- [StateGraph vs FsmStrategy](../explanation/graph-vs-agent.md) — when to use the state machine vs a graph pipeline
- [FSM Strategy Design](../explanation/agent-core-fsm-strategy-design.md) — guard semantics and transition priority

> **Background**: [AI Workflows vs AI Agents](https://www.promptingguide.ai/agents/ai-workflows-vs-ai-agents) — when structured workflows outperform unconstrained agents.

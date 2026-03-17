# How to: Configure Three-Tier Signal Routing

**Goal:** Define signal routes across strategy, agent, and plugin tiers so incoming signals produce the correct agent actions.

---

## Core types

```rust
pub struct Signal {
    pub kind: SignalKind,
    pub payload: serde_json::Value,
}

pub enum SignalKind {
    Stop,
    UserMessage,
    ToolResult,
    Timer,
    Custom(String),
}

pub enum Action {
    Continue,
    GracefulStop,
    ForceStop,
    Transition(String),   // FSM state name
    Custom(String),
}
```

---

## SignalRoute

A route binds a `SignalKind` to an `Action`, with an optional predicate for finer-grained matching and a numeric priority for tie-breaking within the same tier.

```rust
use synwire_core::agents::signal::{Action, Signal, SignalKind, SignalRoute};

// Match all Stop signals.
let route = SignalRoute::new(SignalKind::Stop, Action::GracefulStop, 10);

// Match only non-empty user messages.
fn non_empty(s: &Signal) -> bool {
    s.payload.as_str().is_some_and(|v| !v.is_empty())
}

let guarded = SignalRoute::with_predicate(
    SignalKind::UserMessage,
    non_empty,
    Action::Continue,
    20,  // higher priority — wins over routes with lower values
);
```

Predicates must be plain function pointers (`fn(&Signal) -> bool`) so `SignalRoute` remains `Clone + Send + Sync`.

---

## ComposedRouter

Combines three tiers of routes. Within each tier the highest-priority matching route wins. The strategy tier always beats the agent tier, which always beats the plugin tier — regardless of priority values within tiers.

```rust
use synwire_core::agents::signal::{Action, ComposedRouter, Signal, SignalKind, SignalRoute, SignalRouter};

let strategy_routes = vec![
    // Unconditionally force-stop on Stop signal from strategy level.
    SignalRoute::new(SignalKind::Stop, Action::ForceStop, 0),
];

let agent_routes = vec![
    // Agent prefers graceful stop with higher intra-tier priority.
    SignalRoute::new(SignalKind::Stop, Action::GracefulStop, 100),
    SignalRoute::new(SignalKind::UserMessage, Action::Continue, 0),
    SignalRoute::new(SignalKind::Timer, Action::Transition("tick".to_string()), 0),
];

let plugin_routes = vec![
    SignalRoute::new(SignalKind::Custom("metrics".to_string()), Action::Continue, 0),
];

let router = ComposedRouter::new(strategy_routes, agent_routes, plugin_routes);
```

Routing a signal:

```rust
use serde_json::json;

let signal = Signal::new(SignalKind::Stop, json!(null));

match router.route(&signal) {
    Some(Action::ForceStop)   => { /* strategy tier matched */ }
    Some(Action::GracefulStop) => { /* agent tier matched */ }
    Some(action)              => { /* other action */ }
    None                      => { /* no route — apply default behaviour */ }
}
```

Inspect all registered routes across tiers:

```rust
let all_routes = router.routes();
println!("{} routes registered", all_routes.len());
```

---

## Custom routers

Implement `SignalRouter` to replace the composed approach entirely:

```rust
use synwire_core::agents::signal::{Action, Signal, SignalRouter, SignalRoute};

struct AlwaysContinueRouter;

impl SignalRouter for AlwaysContinueRouter {
    fn route(&self, _signal: &Signal) -> Option<Action> {
        Some(Action::Continue)
    }

    fn routes(&self) -> Vec<SignalRoute> {
        Vec::new()
    }
}
```

---

## Priority semantics summary

| Situation | Winner |
|-----------|--------|
| Strategy route vs. agent route (same kind) | Strategy, regardless of priority values |
| Agent route vs. plugin route (same kind) | Agent, regardless of priority values |
| Two routes in the same tier, same kind | Highest `priority` value |
| Predicate fails on higher-priority route | Next matching route in same tier |
| No route matches in any tier | `None` — caller decides default |

---

**See also**

- [How to: Configure Permission Modes](permission-modes.md)
- [Explanation: Architecture](../explanation/architecture.md)

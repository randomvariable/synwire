# synwire-orchestrator

Synwire's graph-based orchestration engine. Build stateful multi-node workflows using a Pregel superstep execution model.

## What this crate provides

- **`StateGraph<S>`** — define nodes, edges, and conditional routing; compile to an executable graph
- **`CompiledGraph<S>`** — run the graph with `invoke`, stream events with `stream`, checkpoint with `with_checkpoint_saver`
- **`State` trait** — implemented via `#[derive(State)]` for typed state structs
- **Channels** — `LastValue`, `Topic`, `Ephemeral`, `BinaryOperator`, `NamedBarrier`, `AnyValue`; control how state merges across supersteps
- **`ValueState`** — schema-less JSON state for dynamic workflows
- **`sync_node()`** — wrap a synchronous function as a graph node

## Quick start

```toml
[dependencies]
synwire-orchestrator = "0.1"
synwire-derive = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

Define typed state, build a graph, and run it:

```rust,no_run
use synwire_derive::State;
use synwire_orchestrator::graph::{StateGraph, CompiledGraph};
use synwire_orchestrator::constants::END;
use synwire_orchestrator::func::sync_node;

#[derive(State, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct MyState {
    #[reducer(topic)]
    messages: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut graph = StateGraph::<MyState>::new();

    graph.add_node("greet", sync_node(|mut s: MyState| {
        s.messages.push("Hello from the graph!".to_string());
        Ok(s)
    }))?;

    graph.set_entry_point("greet").add_edge("greet", END);

    let compiled = graph.compile()?;
    let result = compiled.invoke(MyState::default(), None).await?;
    println!("{:?}", result.messages);
    Ok(())
}
```

Conditional routing based on state:

```rust,no_run
use synwire_orchestrator::graph::StateGraph;
use synwire_orchestrator::constants::{END, START};

// graph.add_conditional_edges(
//     "classifier",
//     |state: &MyState| {
//         if state.needs_tool { "tool_node" } else { END }
//     },
//     vec!["tool_node", END],
// );
```

## Channel selection

| Scenario | Use |
|---|---|
| Current node name, boolean flag | `LastValue` (default) |
| Message history, event log | `Topic` (appends each update) |
| Per-step scratch data | `Ephemeral` (cleared after superstep) |
| Running counter, custom set merge | `BinaryOperator` |
| Fan-in: wait for N upstream nodes | `NamedBarrier` |
| Heterogeneous / dynamic fields | `AnyValue` |

## Documentation

- [When to use StateGraph vs FsmStrategy](https://randomvariable.github.io/synwire/explanation/graph-vs-agent.html)
- [Pregel Execution Model](https://randomvariable.github.io/synwire/explanation/pregel.html)
- [Orchestrator Explanation](https://randomvariable.github.io/synwire/explanation/synwire-orchestrator.html)
- [Full API docs](https://docs.rs/synwire-orchestrator)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

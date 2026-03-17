# synwire-orchestrator: Graph-Based Multi-Node Workflows

`synwire-orchestrator` provides `StateGraph<S>` — a stateful, compiled graph that runs using a Pregel superstep execution model. Use it when your application has **multiple distinct processing components** that exchange state.

> **Background**: [AI Workflows vs AI Agents](https://www.promptingguide.ai/agents/ai-workflows-vs-ai-agents) — the Prompt Engineering Guide explains when structured workflows outperform autonomous agents. `StateGraph` implements the workflow end of this spectrum.

## When to use `StateGraph`

Use `StateGraph` when you have **≥ 2 distinct roles** in your application that process and pass data to each other:

- LLM call → tool execution → validator → response formatter
- Query classifier → retriever → re-ranker → answer generator
- Draft generator → critic → rewriter → publisher

If you have a **single agent with complex internal turn logic**, use `FsmStrategy` in `synwire-agent` instead. See [StateGraph vs FsmStrategy](./graph-vs-agent.md).

## Building a graph

```rust,no_run
use synwire_derive::State;
use synwire_orchestrator::graph::StateGraph;
use synwire_orchestrator::constants::END;
use synwire_orchestrator::func::sync_node;
use serde::{Serialize, Deserialize};

#[derive(State, Clone, Debug, Default, Serialize, Deserialize)]
struct RagState {
    #[reducer(last_value)]
    query: String,
    #[reducer(topic)]
    context_docs: Vec<String>,
    #[reducer(last_value)]
    answer: String,
}

let mut graph = StateGraph::<RagState>::new();

graph.add_node("retrieve", sync_node(|mut s: RagState| {
    // fetch documents matching s.query
    s.context_docs.push("Rust ownership means one owner at a time.".to_string());
    Ok(s)
}))?;

graph.add_node("generate", sync_node(|mut s: RagState| {
    s.answer = format!("Given: {:?}\nAnswer: …", s.context_docs);
    Ok(s)
}))?;

graph.set_entry_point("retrieve")
    .add_edge("retrieve", "generate")
    .add_edge("generate", END);

let compiled = graph.compile()?;
let result = compiled.invoke(RagState { query: "ownership".into(), ..Default::default() }, None).await?;
println!("{}", result.answer);
```

## Conditional routing

`add_conditional_edges` routes to different nodes based on the current state:

```rust,no_run
use synwire_orchestrator::constants::{END};

// After "classify", route to "tool_node" or directly to END:
// graph.add_conditional_edges(
//     "classify",
//     |state: &MyState| -> &str {
//         if state.needs_tool { "tool_node" } else { END }
//     },
//     vec!["tool_node", END],
// );
```

## Channels: controlling state merging

Each field in a `State` struct has a **channel type** that determines how writes from concurrent nodes are merged:

> 📖 **Rust note:** The `#[derive(State)]` macro (from `synwire-derive`) reads the `#[reducer(...)]` attribute on each field and generates the `State` trait implementation, including `channels()` which returns the channel type for each field.

| Channel | Attribute | Behaviour | Use when |
|---|---|---|---|
| `LastValue` | `#[reducer(last_value)]` or omitted | Overwrites on each write | Current node name, flags, scalars |
| `Topic` | `#[reducer(topic)]` | Appends; accumulates across steps | Message history, event logs |
| `Ephemeral` | `#[reducer(ephemeral)]` | Cleared after each superstep | Per-step scratch data |
| `BinaryOperator` | manual `impl State` | Custom reducer function | Counters, set union, custom merges |
| `NamedBarrier` | manual `impl State` | Fan-in: waits for all named producers | Synchronising parallel branches |
| `AnyValue` | N/A | Accepts any JSON value | Dynamic / schema-less fields |

## Checkpointing

Wire a checkpoint saver to make runs resumable:

```rust,no_run
use synwire_checkpoint::InMemoryCheckpointSaver;
use synwire_checkpoint::CheckpointConfig;
use std::sync::Arc;

let saver = Arc::new(InMemoryCheckpointSaver::new());
let graph = compiled.with_checkpoint_saver(saver);

// First run — thread "session-1" is snapshotted after each superstep
let config = CheckpointConfig::new("session-1");
graph.invoke(RagState::default(), Some(config.clone())).await?;

// Resume from the last checkpoint — same config, new invoke
graph.invoke(RagState::default(), Some(config)).await?;
```

For persistence across process restarts, swap in `SqliteSaver`. See [Checkpointing Tutorial](../tutorials/06-checkpointing.md) and [synwire-checkpoint-sqlite](./synwire-checkpoint-sqlite.md).

## Schema-less state with `ValueState`

If you don't want a typed state struct, use `ValueState` — a wrapper around `serde_json::Value`:

```rust,no_run
use synwire_orchestrator::graph::{StateGraph, ValueState};

let mut graph = StateGraph::<ValueState>::new();
// nodes receive and return ValueState; access fields via .0["field_name"]
```

## See also

- [StateGraph vs FsmStrategy](./graph-vs-agent.md) — when to use which
- [Pregel Execution Model](./pregel.md) — superstep mechanics
- [Channel System](./channels.md) — channel types in depth
- [Checkpointing Explanation](./synwire-checkpoint.md)
- [Graph Agent Getting Started](../getting-started/graph-agent.md)

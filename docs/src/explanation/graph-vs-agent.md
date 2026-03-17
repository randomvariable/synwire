# StateGraph vs FsmStrategy: Choosing the Right Tool

Synwire provides two mechanisms for structured agent behaviour: `StateGraph` from `synwire-orchestrator` and `FsmStrategy` from `synwire-agent`. They solve different problems and are composable — a graph node can itself be an FSM-governed agent.

> **Background**: [AI Workflows vs AI Agents](https://www.promptingguide.ai/agents/ai-workflows-vs-ai-agents) — the Prompt Engineering Guide explains the spectrum from deterministic pipelines to autonomous agents. Both `StateGraph` and `FsmStrategy` sit on this spectrum, just at different scopes.

## `StateGraph`: Multi-node pipelines

Use `StateGraph` when your application has **multiple distinct processing components** that exchange state.

```rust,no_run
use synwire_derive::State;
use synwire_orchestrator::graph::StateGraph;
use synwire_orchestrator::constants::END;
use synwire_orchestrator::func::sync_node;

#[derive(State, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct PipelineState {
    #[reducer(last_value)]
    query: String,
    #[reducer(topic)]
    retrieved_docs: Vec<String>,
    #[reducer(last_value)]
    answer: String,
}

// Three distinct nodes: retrieve → generate → format
let mut graph = StateGraph::<PipelineState>::new();
graph.add_node("retrieve", sync_node(|mut s: PipelineState| {
    s.retrieved_docs.push("Doc about Rust ownership".to_string());
    Ok(s)
}))?;
graph.add_node("generate", sync_node(|mut s: PipelineState| {
    s.answer = format!("Based on {} docs: …", s.retrieved_docs.len());
    Ok(s)
}))?;
graph.add_node("format", sync_node(|mut s: PipelineState| {
    s.answer = format!("**Answer**: {}", s.answer);
    Ok(s)
}))?;

graph.set_entry_point("retrieve")
    .add_edge("retrieve", "generate")
    .add_edge("generate", "format")
    .add_edge("format", END);

let compiled = graph.compile()?;
```

**What `StateGraph` gives you:**
- A **compiled, static topology** — nodes and edges are fixed at compile time; the Pregel engine validates the graph before execution
- **Channel-based state merging** — each field in `State` has an explicit merge rule (`LastValue`, `Topic`, `Ephemeral`, etc.); concurrent node writes are safe and deterministic
- **Conditional routing** — `add_conditional_edges` routes to different nodes based on state; enables branching and retry loops
- **Checkpointing** — `with_checkpoint_saver` snapshots state after each superstep; runs are resumable

## `FsmStrategy`: One agent's internal turn logic

Use `FsmStrategy` when **a single agent** needs structured state machine semantics governing its own turn sequence.

```rust,no_run
use synwire_agent::strategies::{FsmStrategyBuilder, ClosureGuard};
use synwire_core::agents::directive::Directive;

let strategy = FsmStrategyBuilder::new()
    .add_state("waiting")
    .add_state("executing")
    .add_state("reviewing")
    .set_initial_state("waiting")
    // Transition to executing when the agent issues a RunInstruction directive
    .add_transition(
        "waiting",
        "executing",
        ClosureGuard::new(|d| matches!(d, Directive::RunInstruction { .. })),
    )
    // Transition to reviewing after execution completes
    .add_transition("executing", "reviewing", ClosureGuard::always())
    // Return to waiting after review
    .add_transition("reviewing", "waiting", ClosureGuard::always())
    .build()?;
```

**What `FsmStrategy` gives you:**
- A **runtime transition table** — states and transitions are evaluated on every turn
- **Guard conditions** — closures over `Directive` values decide whether a transition fires; guards can be priority-ordered
- **Approval gate integration** — a guard can check `RiskLevel` and block a transition until human approval arrives
- **No topology knowledge** — `FsmStrategy` operates entirely within one `Runner`'s turn loop; it has no notion of other nodes or channels

## Decision table

| Dimension | `StateGraph` | `FsmStrategy` |
|---|---|---|
| **Scope** | Multiple distinct system components (LLM, retriever, validator, formatter) | Single agent's internal turn logic |
| **State sharing** | Explicit channels; nodes exchange structured state | Agent's own `State` type; not shared across nodes |
| **Routing** | Conditional edges defined at graph build time | Guard conditions evaluated at runtime per directive |
| **Checkpointing** | First-class, per-superstep | Not built-in (session management handles persistence) |
| **Concurrency** | Parallel node execution within a superstep | Sequential turns within one `Runner` |
| **When to choose** | You have ≥ 2 distinct processing roles | You have 1 agent needing structured turn logic |

## They compose

`StateGraph` and `FsmStrategy` are **not mutually exclusive**. A graph node can be a `Runner` backed by `FsmStrategy`:

```rust,no_run
use synwire_agent::runner::Runner;
use synwire_agent::strategies::FsmStrategy;
use synwire_orchestrator::graph::StateGraph;

// An FSM-governed agent used as one node in a larger graph.
// The graph handles multi-node orchestration;
// the FsmStrategy handles the agent's internal turn structure.

// let fsm_agent = Runner::builder()
//     .agent(my_agent_node)
//     .strategy(my_fsm_strategy)
//     .build()?;
//
// graph.add_node("agent_step", async move |state| {
//     let events = fsm_agent.run(state.input.clone(), Default::default()).await?;
//     // ... collect events, update state
//     Ok(state)
// });
```

## See also

- [FSM Strategy Design](./agent-core-fsm-strategy-design.md) — guard conditions, priority, transition semantics
- [Pregel Execution Model](./pregel.md) — how `StateGraph` executes supersteps
- [Execution Strategies Tutorial](../tutorials/03-execution-strategies.md) — hands-on with `DirectStrategy` and `FsmStrategy`
- [Graph Agent Getting Started](../getting-started/graph-agent.md) — your first `StateGraph`

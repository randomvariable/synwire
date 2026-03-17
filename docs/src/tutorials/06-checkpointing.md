# Tutorial 6: Checkpointing — Resumable Workflows

**Prerequisites**: Rust 1.85+, completion of [Tutorial 5](./05-backend-operations.md), familiarity with `StateGraph` from [Getting Started: Graph Agents](../getting-started/graph-agent.md).

In this tutorial you will:

1. Understand what checkpointing does and when you need it
2. Wire `InMemoryCheckpointSaver` into a `StateGraph`
3. Resume a run from a checkpoint using `thread_id`
4. Switch to `SqliteSaver` for persistence across process restarts
5. Fork a run from a past checkpoint

---

## 1. Why checkpoint?

Without checkpointing, every `graph.invoke(...)` starts from scratch. Checkpointing enables:

- **Resume** — a long-running workflow interrupted mid-way (network error, process restart) picks up from the last completed superstep
- **Fork** — run alternative continuations from the same past state without re-executing earlier steps
- **Replay / debug** — rewind to an intermediate state and inspect what changed
- **Human-in-the-loop** — pause execution at a decision point, wait for human input, then resume

> 📖 **Rust note:** [`Arc<T>`](https://doc.rust-lang.org/std/sync/struct.Arc.html) (Atomically Reference Counted) enables shared ownership of a value across threads. We use `Arc<dyn BaseCheckpointSaver>` because the compiled graph and the caller both need access to the saver.

---

## 2. In-memory checkpointing

Add dependencies:

```toml
[dependencies]
synwire-orchestrator = "0.1"
synwire-checkpoint = "0.1"
synwire-derive = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

Define state and a simple two-step graph:

```rust,no_run
use synwire_derive::State;
use synwire_orchestrator::graph::StateGraph;
use synwire_orchestrator::constants::END;
use synwire_orchestrator::func::sync_node;
use synwire_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig};
use std::sync::Arc;
use serde::{Serialize, Deserialize};

#[derive(State, Clone, Debug, Default, Serialize, Deserialize)]
struct WorkflowState {
    #[reducer(topic)]
    steps_completed: Vec<String>,
    #[reducer(last_value)]
    result: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut graph = StateGraph::<WorkflowState>::new();

    graph.add_node("step_one", sync_node(|mut s: WorkflowState| {
        s.steps_completed.push("step_one".to_string());
        Ok(s)
    }))?;

    graph.add_node("step_two", sync_node(|mut s: WorkflowState| {
        s.steps_completed.push("step_two".to_string());
        s.result = format!("Completed: {:?}", s.steps_completed);
        Ok(s)
    }))?;

    graph.set_entry_point("step_one")
        .add_edge("step_one", "step_two")
        .add_edge("step_two", END);

    let compiled = graph.compile()?;

    // Attach the saver
    let saver = Arc::new(InMemoryCheckpointSaver::new());
    let checkpointed = compiled.with_checkpoint_saver(saver.clone());

    // Run with thread_id "session-1"
    let config = CheckpointConfig::new("session-1");
    let state = checkpointed.invoke(WorkflowState::default(), Some(config)).await?;
    println!("Result: {}", state.result);

    Ok(())
}
```

After `invoke` completes, `saver` holds a checkpoint for `"session-1"`. The checkpoint contains the full state after each superstep.

---

## 3. Resuming from a checkpoint

Pass the same `thread_id` on a subsequent call. The graph loads the latest checkpoint and continues from there:

```rust,no_run
// ... (same graph and saver from above)

// First run
let config = CheckpointConfig::new("my-thread");
checkpointed.invoke(WorkflowState::default(), Some(config.clone())).await?;

// Simulate an interruption here. On resume:
let resumed = checkpointed.invoke(WorkflowState::default(), Some(config)).await?;
println!("Resumed result: {}", resumed.result);
// The graph finds the existing checkpoint and skips already-completed supersteps.
```

> **Note**: `InMemoryCheckpointSaver` loses all state when the process exits. For true resumability across restarts, use `SqliteSaver` (next section).

---

## 4. SQLite checkpointing for durable persistence

Add `synwire-checkpoint-sqlite`:

```toml
[dependencies]
synwire-checkpoint-sqlite = "0.1"
# ... rest unchanged
```

Replace `InMemoryCheckpointSaver` with `SqliteSaver`:

```rust,no_run
use synwire_checkpoint_sqlite::SqliteSaver;
use std::path::Path;
use std::sync::Arc;

// Opens or creates "checkpoints.db" in the current directory.
// File permissions are set to 0600 automatically.
let saver = Arc::new(SqliteSaver::new(Path::new("checkpoints.db"))?);

let checkpointed = compiled.with_checkpoint_saver(saver);
let config = CheckpointConfig::new("persistent-session");
checkpointed.invoke(WorkflowState::default(), Some(config.clone())).await?;

// Kill the process here. On restart:
// The same code opens "checkpoints.db" and resumes from the last superstep.
let resumed = checkpointed.invoke(WorkflowState::default(), Some(config)).await?;
```

No system SQLite library is required — `synwire-checkpoint-sqlite` bundles SQLite via the `rusqlite` `bundled` feature.

---

## 5. Forking from a past checkpoint

To fork at a specific checkpoint, provide its `checkpoint_id`:

```rust,no_run
use synwire_checkpoint::{CheckpointConfig, BaseCheckpointSaver};

// List all checkpoints for a thread
let checkpoints = saver.list(&CheckpointConfig::new("my-thread"), None).await?;

// Fork from the first checkpoint (earliest in the run)
if let Some(first) = checkpoints.first() {
    let fork_config = CheckpointConfig::new("my-thread")
        .with_checkpoint_id(first.checkpoint.id.clone());

    let forked = checkpointed.invoke(WorkflowState::default(), Some(fork_config)).await?;
    println!("Fork result: {}", forked.result);
}
```

The forked run creates a new branch in the checkpoint tree, identified by a new `checkpoint_id`. The original thread remains unchanged.

---

## Next steps

- [How-To: Add Checkpointing](../how-to/add-checkpointing.md) — configuration options and advanced patterns
- [Checkpointing Explanation](../explanation/synwire-checkpoint.md) — `BaseStore` and the serde protocol
- [Pregel Execution Model](../explanation/pregel.md) — how supersteps relate to checkpoints

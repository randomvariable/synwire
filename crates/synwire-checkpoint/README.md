# synwire-checkpoint

Checkpoint persistence traits for Synwire graphs. Enables resumable, forkable, and rewindable workflow runs.

## What this crate provides

- **`BaseCheckpointSaver`** — trait for saving and loading graph snapshots: `get_tuple`, `list`, `put`
- **`InMemoryCheckpointSaver`** — zero-config, process-lifetime checkpoint store
- **`BaseStore`** — general-purpose K-V store for agent state persistence: `get`, `put`, `delete`, `list`
- **`CheckpointConfig`** — `thread_id` namespaces runs; `checkpoint_id` targets a specific snapshot
- **`CheckpointTuple`** — a checkpoint with its config, metadata, and parent reference
- **`Checkpoint`** — snapshot of all channel values at a given step
- **`CheckpointMetadata`** — step number, source, writes, parent references

## Quick start

```toml
[dependencies]
synwire-checkpoint = "0.1"
synwire-orchestrator = "0.1"
tokio = { version = "1", features = ["full"] }
```

Wire `InMemoryCheckpointSaver` into a compiled graph:

```rust,no_run
use synwire_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig};
use synwire_orchestrator::graph::ValueState;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // assume `compiled` is a CompiledGraph<ValueState>
    // let saver = Arc::new(InMemoryCheckpointSaver::new());
    // let graph = compiled.with_checkpoint_saver(saver);
    //
    // First run — thread_id "session-1" is checkpointed
    // let config = CheckpointConfig::new("session-1");
    // let result = graph.invoke(ValueState::default(), Some(config.clone())).await?;
    //
    // Resume the same run later — graph picks up from the last checkpoint
    // let resumed = graph.invoke(ValueState::default(), Some(config)).await?;
    Ok(())
}
```

## Resuming from a checkpoint

Pass the same `thread_id` on subsequent `invoke` calls. The graph loads the latest checkpoint for that thread and continues from where it left off.

For durable storage across process restarts, use [`synwire-checkpoint-sqlite`](https://docs.rs/synwire-checkpoint-sqlite).

## Documentation

- [Checkpointing Tutorial](https://randomvariable.github.io/synwire/tutorials/06-checkpointing.html)
- [How-To: Add Checkpointing](https://randomvariable.github.io/synwire/how-to/add-checkpointing.html)
- [Checkpoint Explanation](https://randomvariable.github.io/synwire/explanation/synwire-checkpoint.html)
- [Full API docs](https://docs.rs/synwire-checkpoint)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

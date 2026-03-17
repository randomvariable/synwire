# synwire-checkpoint: Persistence and Resumability

`synwire-checkpoint` provides two persistence mechanisms:

1. **`BaseCheckpointSaver`** — snapshots `StateGraph` runs so they can be resumed, forked, or rewound
2. **`BaseStore`** — general key-value storage for agent state that must outlive a single turn

## `BaseCheckpointSaver` — graph snapshots

Every time a `CompiledGraph` completes a superstep, it can save a `Checkpoint` containing the full channel state. The next `invoke` call with the same `thread_id` loads the latest checkpoint and resumes from there.

```rust,no_run
use synwire_checkpoint::{BaseCheckpointSaver, InMemoryCheckpointSaver, CheckpointConfig};
use std::sync::Arc;

// In-memory — zero config, process-lifetime only
let saver: Arc<dyn BaseCheckpointSaver> = Arc::new(InMemoryCheckpointSaver::new());

// Namespace runs by thread_id
let config = CheckpointConfig::new("user-session-42");

// Run 1 — state is saved after each superstep
// compiled.with_checkpoint_saver(saver.clone()).invoke(state, Some(config.clone())).await?;

// Run 2 — resumes from the last saved step
// compiled.with_checkpoint_saver(saver).invoke(state, Some(config)).await?;
```

### What a checkpoint contains

- **`CheckpointConfig`** — `thread_id` (namespace) + optional `checkpoint_id` (specific snapshot)
- **`Checkpoint`** — `id`, `channel_values` (full state), `format_version`
- **`CheckpointMetadata`** — `source` (how it was created), `step` (superstep number), `writes` (what changed), `parents` (for forking)
- **`CheckpointTuple`** — checkpoint + config + metadata + parent config

### Forking and rewinding

`CheckpointMetadata.parents` links each snapshot to its predecessor, forming a tree. To fork from a past checkpoint, pass its `checkpoint_id` in `CheckpointConfig`. The graph resumes from that snapshot, creating a new branch.

## `BaseStore` — general K-V persistence

`BaseStore` is a simpler interface for ad-hoc key-value storage — useful for caching tool results, storing agent memory, or persisting data between sessions without the overhead of full graph snapshotting.

```rust,no_run
use synwire_checkpoint::BaseStore;

// Assume `store` implements BaseStore:
// store.put("agent:memory:user-42", serde_json::json!({ "name": "Alice" }))?;
// let mem = store.get("agent:memory:user-42")?;
```

## Choosing a saver

| Saver | Crate | Persistence | Use when |
|---|---|---|---|
| `InMemoryCheckpointSaver` | `synwire-checkpoint` | Process-lifetime | Tests, short workflows |
| `SqliteSaver` | `synwire-checkpoint-sqlite` | Disk, survives restarts | Single-process production |
| Custom `BaseCheckpointSaver` | Your crate | PostgreSQL, Redis, S3, … | Distributed or multi-process |

## Serde protocol

Checkpoints serialise channel values to JSON using `JsonPlusSerde`. Any custom type stored in a channel must implement `serde::Serialize` and `serde::Deserialize`.

```rust,no_run
use serde::{Serialize, Deserialize};

// Any type stored in a StateGraph channel needs these derives:
#[derive(Serialize, Deserialize, Clone, Debug)]
struct MyChannelValue {
    content: String,
    step: u32,
}
```

## When NOT to checkpoint

Checkpointing has a cost: each superstep writes serialised state to storage. Avoid it for:

- **Stateless request/response** — single-turn LLM calls with no need to resume
- **Short-lived workflows** — complete in one process lifetime with no user-visible progress state
- **High-frequency loops** — hundreds of supersteps per second where checkpoint I/O would dominate latency

## See also

- [Checkpointing Tutorial](../tutorials/06-checkpointing.md)
- [How-To: Add Checkpointing](../how-to/add-checkpointing.md)
- [synwire-checkpoint-sqlite](./synwire-checkpoint-sqlite.md)
- [Pregel Execution Model](./pregel.md)

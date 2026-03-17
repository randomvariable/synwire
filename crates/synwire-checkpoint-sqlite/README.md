# synwire-checkpoint-sqlite

SQLite-backed checkpoint storage for Synwire graphs. Persist workflow snapshots across process restarts with zero system dependencies — SQLite is bundled.

## What this crate provides

- **`SqliteSaver`** — implements `BaseCheckpointSaver` with WAL-mode SQLite persistence
- **`SqliteSaver::new(path)`** — open or create a checkpoint database (permissions: 0600)
- **`SqliteSaver::with_max_size(path, max_bytes)`** — cap the stored checkpoint JSON size
- Connection pooling via `r2d2` for safe concurrent access
- No `libsqlite3` system dependency — uses `rusqlite` with the `bundled` feature

## Quick start

```toml
[dependencies]
synwire-checkpoint-sqlite = "0.1"
synwire-checkpoint = "0.1"
synwire-orchestrator = "0.1"
tokio = { version = "1", features = ["full"] }
```

Create a persistent checkpoint saver and wire it into a graph:

```rust,no_run
use synwire_checkpoint_sqlite::SqliteSaver;
use synwire_checkpoint::CheckpointConfig;
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let saver = Arc::new(SqliteSaver::new(Path::new("checkpoints.db"))?);

    // assume `compiled` is a CompiledGraph<S>
    // let graph = compiled.with_checkpoint_saver(saver);
    // let config = CheckpointConfig::new("my-thread");
    //
    // Run 1 — state is persisted to checkpoints.db
    // graph.invoke(state, Some(config.clone())).await?;
    //
    // Process restarts here — then run 2 resumes transparently
    // graph.invoke(state, Some(config)).await?;
    Ok(())
}
```

## When to use this vs InMemoryCheckpointSaver

| Scenario | Use |
|---|---|
| Tests, short-lived workflows | `InMemoryCheckpointSaver` |
| Persistence across process restarts | `SqliteSaver` |
| Distributed / multi-process | Implement `BaseCheckpointSaver` for PostgreSQL, Redis, etc. |

## Documentation

- [Checkpointing Tutorial](https://randomvariable.github.io/synwire/tutorials/06-checkpointing.html)
- [`synwire-checkpoint`](https://docs.rs/synwire-checkpoint) — trait definitions
- [Full API docs](https://docs.rs/synwire-checkpoint-sqlite)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

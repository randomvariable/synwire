# synwire-checkpoint-sqlite: SQLite Checkpoint Backend

`synwire-checkpoint-sqlite` provides `SqliteSaver`, a production-ready `BaseCheckpointSaver` implementation backed by SQLite. It persists graph execution checkpoints to a local database file with security-conscious defaults and configurable size limits.

## Why SQLite?

Checkpointing needs durable, ordered storage with transactional guarantees. SQLite provides all of this in a single file with no external daemon, no network round-trips, and no configuration beyond a file path. For local agent workloads --- where checkpoints are written by one process and read back by the same process or a restart of it --- SQLite is the simplest backend that is also correct.

WAL (Write-Ahead Logging) mode is used for concurrency. Multiple readers can proceed in parallel with a single writer, which matches the typical checkpoint access pattern: frequent reads during graph traversal, occasional writes at step boundaries.

## Key type: `SqliteSaver`

```rust,no_run
use std::path::Path;
use synwire_checkpoint_sqlite::saver::SqliteSaver;

// Default: 16 MiB max checkpoint size
let saver = SqliteSaver::new(Path::new("/tmp/checkpoints.db"))?;

// Custom size limit
let saver = SqliteSaver::with_max_size(Path::new("/tmp/checkpoints.db"), 4 * 1024 * 1024)?;
```

`SqliteSaver` implements `BaseCheckpointSaver` with three operations:

| Method | Behaviour |
|---|---|
| `put` | Serializes the checkpoint to JSON, enforces `max_checkpoint_size`, records the parent chain, and inserts via `INSERT OR REPLACE` |
| `get_tuple` | Retrieves the latest checkpoint for a thread (or a specific checkpoint by ID) |
| `list` | Returns checkpoints for a thread in reverse chronological order, with optional limit |

### Security: file permissions

On Unix systems, `SqliteSaver::new` creates the database file with mode `0600` (owner read/write only) *before* handing it to SQLite. This prevents other users on a shared machine from reading checkpoint data, which may contain agent conversation history, tool call results, or application state.

### Size limits

The `max_checkpoint_size` parameter (default 16 MiB) is enforced on every `put`. If a serialized checkpoint exceeds the limit, `put` returns `CheckpointError::StateTooLarge` without writing to the database. This prevents runaway state growth from consuming disk space, which can happen when an agent accumulates large tool outputs across many steps.

## Connection pooling

`SqliteSaver` uses `r2d2` with `r2d2_sqlite` for connection pooling. The pool is configured with a maximum of 4 connections, which is sufficient for the single-writer/multiple-reader pattern. The pool is wrapped in an `Arc` so `SqliteSaver` is cheaply cloneable.

## Schema

The database has a single table:

```sql
CREATE TABLE IF NOT EXISTS checkpoints (
    thread_id              TEXT NOT NULL,
    checkpoint_id          TEXT NOT NULL,
    data                   BLOB NOT NULL,
    metadata               TEXT NOT NULL,
    parent_checkpoint_id   TEXT,
    PRIMARY KEY (thread_id, checkpoint_id)
);
```

Checkpoints are stored as JSON-serialized blobs. Metadata is stored as a separate JSON text column to allow querying without deserializing the full checkpoint.

## Dependencies

| Crate | Role |
|---|---|
| `synwire-checkpoint` | `BaseCheckpointSaver` trait and checkpoint types |
| `synwire-core` | `BoxFuture` for async trait methods |
| `rusqlite` | SQLite bindings |
| `r2d2` / `r2d2_sqlite` | Connection pooling |
| `serde_json` | Checkpoint serialization |
| `thiserror` | Error types |

## Ecosystem position

`SqliteSaver` is the recommended checkpoint backend for single-machine deployments. For distributed systems where multiple processes need to share checkpoint state, a networked backend (Redis, PostgreSQL) would be implemented as a separate crate, using the same `BaseCheckpointSaver` trait and validated by the same conformance suite.

```text
synwire-checkpoint            (trait)
    |
    +-- synwire-checkpoint-sqlite   (this crate: SqliteSaver)
    +-- synwire-checkpoint-conformance (test suite)
```

## See also

- [synwire-checkpoint: Persistence](./synwire-checkpoint.md) --- the trait definition
- [synwire-checkpoint-conformance](./synwire-checkpoint-conformance.md) --- the conformance test suite
- [Add Checkpointing](../how-to/add-checkpointing.md) --- how-to guide
- [Checkpointing](../tutorials/06-checkpointing.md) --- tutorial

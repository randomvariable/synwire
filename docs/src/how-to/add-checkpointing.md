# Add Checkpointing

Checkpointing persists graph state between supersteps, enabling pause/resume and state inspection.

## In-memory checkpointing

For development and testing:

```rust,ignore
use synwire_checkpoint::memory::InMemoryCheckpointSaver;

let saver = InMemoryCheckpointSaver::new();
```

## SQLite checkpointing

For persistent storage:

```rust,ignore
use synwire_checkpoint_sqlite::saver::SqliteSaver;

let saver = SqliteSaver::new("checkpoints.db")?;
```

## BaseCheckpointSaver trait

All checkpoint implementations satisfy `BaseCheckpointSaver`:

```rust,ignore
use synwire_checkpoint::base::BaseCheckpointSaver;

async fn save_state(
    saver: &dyn BaseCheckpointSaver,
    thread_id: &str,
    state: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    // Save checkpoint...
    Ok(())
}
```

## Key-value store

For arbitrary key-value storage alongside checkpoints, use `BaseStore`:

```rust,ignore
use synwire_checkpoint::store::base::BaseStore;
```

## Custom checkpoint backend

Implement `BaseCheckpointSaver` for your own storage backend. Use the conformance test suite to validate:

```rust,ignore
use synwire_checkpoint_conformance::run_conformance_tests;

// In a test:
run_conformance_tests(|| your_saver_factory()).await;
```

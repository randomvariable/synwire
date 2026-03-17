# synwire-checkpoint-conformance

Conformance test suite for Synwire checkpoint implementations. Call `run_conformance_tests()` against any `BaseCheckpointSaver` to verify it satisfies the checkpoint protocol specification.

## What this crate provides

- **`run_conformance_tests()`** -- exercises every `BaseCheckpointSaver` contract (put, get, list, delete, concurrency invariants)
- **Protocol-level validation** -- ensures checkpoint metadata, ordering, and deduplication behave correctly
- **Backend-agnostic** -- works with SQLite, in-memory, or any future checkpoint backend
- **Zero unsafe code** -- `#![deny(unsafe_code)]`

## Quick start

```toml
[dev-dependencies]
synwire-checkpoint-conformance = { path = "../synwire-checkpoint-conformance" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Run the suite against your backend:

```rust,no_run
use synwire_checkpoint_conformance::run_conformance_tests;

#[tokio::test]
async fn my_backend_conforms() {
    let saver = create_my_checkpoint_saver().await;
    run_conformance_tests(saver).await;
}
```

## Documentation

- [Checkpoint Explanation](https://randomvariable.github.io/synwire/explanation/synwire-checkpoint.html)
- [Full API docs](https://docs.rs/synwire-checkpoint-conformance)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

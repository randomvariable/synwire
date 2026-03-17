# synwire-checkpoint-conformance: Checkpoint Test Suite

`synwire-checkpoint-conformance` provides a reusable conformance test suite that validates any `BaseCheckpointSaver` implementation against the checkpoint protocol specification. If you are writing a custom checkpoint backend --- Redis, PostgreSQL, S3, or anything else --- this crate tells you whether your implementation is correct.

## Why a separate crate?

Checkpoint backends are expected to come from both the Synwire workspace and third-party authors. The conformance tests encode the contract that `BaseCheckpointSaver` must satisfy: ordering guarantees, parent-chain integrity, metadata round-tripping, size limit enforcement, and idempotency. Shipping these tests as a standalone crate means:

1. **Third-party backends** can add `synwire-checkpoint-conformance` as a dev-dependency and run the same test suite that the built-in SQLite backend uses.
2. **The contract is executable.** Instead of relying on prose documentation to define correct behaviour, the conformance suite *is* the specification.
3. **Regressions are caught early.** Any change to checkpoint semantics that breaks the conformance suite is visible across all backends, not just the ones in the workspace.

## How to use it

Add the crate as a dev-dependency in your backend crate:

```toml
[dev-dependencies]
synwire-checkpoint-conformance = { path = "../synwire-checkpoint-conformance" }
tokio = { version = "1", features = ["full"] }
```

Then call the conformance runner from a test:

```rust,no_run
use synwire_checkpoint_conformance::run_conformance_tests;

#[tokio::test]
async fn my_backend_conforms() {
    let saver = MyCustomSaver::new(/* ... */);
    run_conformance_tests(&saver).await;
}
```

The suite exercises all `BaseCheckpointSaver` methods --- `put`, `get_tuple`, `list` --- with a variety of inputs and validates that the results match the expected protocol behaviour.

## What the suite tests

The conformance tests cover the following areas:

| Area | What is validated |
|---|---|
| **Basic CRUD** | `put` stores a checkpoint, `get_tuple` retrieves it, `list` returns all checkpoints for a thread |
| **Ordering** | `get_tuple` with no checkpoint ID returns the most recent checkpoint; `list` returns checkpoints in reverse chronological order |
| **Parent chain** | Each `put` records the previous checkpoint as its parent; `parent_config` is correctly populated |
| **Specific retrieval** | `get_tuple` with a specific checkpoint ID returns exactly that checkpoint |
| **Missing data** | Querying a non-existent thread returns `None`, not an error |
| **List limits** | `list` with a `limit` parameter returns at most that many results |
| **Metadata round-trip** | `CheckpointMetadata` (source, step, writes, parents) survives serialization and deserialization |

## Dependencies

| Crate | Role |
|---|---|
| `synwire-checkpoint` | `BaseCheckpointSaver` trait and checkpoint types |
| `synwire-core` | `BoxFuture` and shared error types |
| `tokio` | Async test runtime |

This is a `publish = false` crate --- it exists only for testing within the workspace and by downstream backends that can reference it via path or git dependency.

## Ecosystem position

```text
synwire-checkpoint          (trait: BaseCheckpointSaver)
    |
    +-- synwire-checkpoint-sqlite     (impl: SqliteSaver)
    |       |
    |       +-- synwire-checkpoint-conformance  (tests)
    |
    +-- your-custom-backend           (impl: YourSaver)
            |
            +-- synwire-checkpoint-conformance  (tests)
```

Every checkpoint backend in the ecosystem should run the conformance suite. The suite is the source of truth for what "correct" means.

## See also

- [synwire-checkpoint: Persistence](./synwire-checkpoint.md) --- the trait definition
- [synwire-checkpoint-sqlite](./synwire-checkpoint-sqlite.md) --- the built-in SQLite backend
- [Add Checkpointing](../how-to/add-checkpointing.md) --- how-to guide

# Offline Usage

Synwire can be used entirely offline without API keys for development, testing, and CI.

## FakeChatModel

Returns pre-configured responses deterministically:

```rust,ignore
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_core::messages::Message;

let model = FakeChatModel::new(vec![
    "First response".into(),
    "Second response".into(),
]);

// Responses cycle: call 0 -> "First response", call 1 -> "Second response", call 2 -> "First response"
```

### Features

- **Error injection**: `with_error_at(n)` returns an error on call `n`
- **Stream chunking**: `with_chunk_size(n)` splits responses into `n`-character chunks
- **Stream errors**: `with_stream_error_after(n)` injects errors after `n` chunks
- **Call tracking**: `call_count()` and `calls()` for assertions

## FakeEmbeddings

Returns deterministic embedding vectors:

```rust,ignore
use synwire_core::embeddings::{FakeEmbeddings, Embeddings};

let embeddings = FakeEmbeddings::new(32); // 32-dimensional vectors
let vectors = embeddings.embed_documents(&["hello".into()]).await?;
```

## InMemoryVectorStore

Full vector store implementation with no external dependencies:

```rust,ignore
use synwire_core::vectorstores::InMemoryVectorStore;

let store = InMemoryVectorStore::new();
```

## InMemoryCheckpointSaver

In-memory checkpoint storage:

```rust,ignore
use synwire_checkpoint::memory::InMemoryCheckpointSaver;

let saver = InMemoryCheckpointSaver::new();
```

## Test utilities

The `synwire-test-utils` crate provides:

- Proptest strategies for all core types (messages, documents, embeddings, tools, channels, graphs)
- Fixture builders (`DocumentBuilder`, `MessageBuilder`, `PromptTemplateBuilder`, `ToolSchemaBuilder`)
- Re-exports of all strategy modules

## CI without API keys

All tests in the workspace run with `cargo nextest run` without any environment variables. Integration tests requiring live APIs are behind feature flags or in separate test files excluded from default runs.

## Disabling network features

For air-gapped environments:

```toml
[dependencies]
synwire-core = { version = "0.1", default-features = false }
```

This removes `reqwest` and `backoff` dependencies. Use `FakeChatModel` and `FakeEmbeddings` exclusively.

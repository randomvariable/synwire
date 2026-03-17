# Dependency Reference

Every significant third-party crate used by Synwire, grouped by concern. When you encounter an unfamiliar import in Synwire source code or a compiler error mentioning a crate you don't recognise, look it up here.

---

## Async runtime

### `tokio`

**docs.rs**: <https://docs.rs/tokio>
**What it is**: The most widely used async runtime for Rust. Provides the thread pool, timers, I/O reactors, channels, and synchronisation primitives that underpin all of Synwire.
**Where you'll see it**: `#[tokio::main]`, `#[tokio::test]`, `tokio::spawn`, `tokio::sync::Mutex`, `tokio::time::sleep`.
**When to use it directly**: Your application entry point needs `#[tokio::main]`; async tests need `#[tokio::test]`. All Synwire async code runs on the tokio executor.

### `futures-core` / `futures-util`

**docs.rs**: <https://docs.rs/futures>
**What it is**: The `Stream` trait and its combinators (`StreamExt`, `FutureExt`, `SinkExt`).
**Where you'll see it**: `BoxStream` in all streaming APIs; `StreamExt::next()` to consume `AgentEvent` streams.
**When to use it directly**: When consuming a `BoxStream` — call `.next().await` or `.for_each(...)`.

### `pin-project-lite`

**docs.rs**: <https://docs.rs/pin-project-lite>
**What it is**: Safe `Pin` projection for custom `Future` and `Stream` implementations.
**Where you'll see it**: Synwire internals for stream adapters; you rarely touch this directly.
**When to use it directly**: When writing a custom `Future` or `Stream` struct that holds pinned fields.

---

## Serialisation

### `serde` + `serde_json`

**docs.rs**: <https://docs.rs/serde>, <https://docs.rs/serde_json>
**What it is**: The de-facto serialisation framework for Rust. `serde` defines `Serialize`/`Deserialize` traits; `serde_json` implements them for JSON.
**Where you'll see it**: `serde_json::Value` is the universal I/O type for `RunnableCore`, `StateGraph` channels, and `Checkpoint` storage. Every Synwire public type derives `Serialize` and `Deserialize`.
**When to use it directly**: Your own state structs and tool parameter types need `#[derive(Serialize, Deserialize)]`.

### `typetag`

**docs.rs**: <https://docs.rs/typetag>
**What it is**: Enables `dyn Serialize` and `dyn Deserialize` for trait objects — normally impossible because trait objects erase type information.
**Where you'll see it**: `Directive::Custom(Box<dyn CustomDirective>)` uses `typetag` to serialise unknown directive types.
**When to use it directly**: When implementing `CustomDirective` for a new directive variant.

### `humantime-serde`

**docs.rs**: <https://docs.rs/humantime-serde>
**What it is**: Serde support for human-readable duration strings (`"30s"`, `"2h"`, `"500ms"`).
**Where you'll see it**: `TimeoutMiddleware`, `RateLimitMiddleware` config structs.
**When to use it directly**: Add `#[serde(with = "humantime_serde")]` to any `Duration` field in your config struct.

### `json-patch`

**docs.rs**: <https://docs.rs/json-patch>
**What it is**: RFC 6902 JSON Patch and RFC 7386 JSON Merge Patch.
**Where you'll see it**: `StateGraph` channel merge operations.
**When to use it directly**: Rarely needed directly.

---

## Error handling

### `thiserror`

**docs.rs**: <https://docs.rs/thiserror>
**What it is**: `#[derive(Error)]` for library error types. Generates `Display` and `From` impls from annotations.
**Where you'll see it**: Every Synwire error enum (`SynwireError`, `AgentError`, `VfsError`, etc.).
**When to use it directly**: Your own extension crates should use `thiserror` for their error types — this ensures errors compose well with Synwire's error hierarchy. Use `anyhow` only in application binaries and tests.

---

## HTTP and streaming

### `reqwest`

**docs.rs**: <https://docs.rs/reqwest>
**What it is**: Async HTTP client with rustls TLS, JSON request/response, streaming, and multipart.
**Where you'll see it**: Used by `synwire-llm-openai`, `synwire-llm-ollama`, and `HttpBackend`.
**When to use it directly**: When implementing a custom LLM provider or `HttpBackend` extension.

### `reqwest-middleware` / `reqwest-retry`

**docs.rs**: <https://docs.rs/reqwest-middleware>
**What it is**: Middleware layer and automatic retry logic for `reqwest`.
**Where you'll see it**: `synwire-llm-openai` wraps its client with exponential backoff retry.
**When to use it directly**: When building a custom HTTP-based backend with retry behaviour.

### `eventsource-stream`

**docs.rs**: <https://docs.rs/eventsource-stream>
**What it is**: Parses Server-Sent Events (SSE) from an async byte stream.
**Where you'll see it**: `ChatOpenAI::stream()` and `ChatOllama::stream()` use this to parse streaming LLM responses.
**When to use it directly**: When implementing a custom SSE-based streaming provider.

### `backoff`

**docs.rs**: <https://docs.rs/backoff>
**What it is**: Exponential backoff with jitter for retry loops.
**Where you'll see it**: `synwire-llm-openai` retry logic.
**When to use it directly**: Custom retry loops in extension crates.

---

## Security / credentials

### `secrecy`

**docs.rs**: <https://docs.rs/secrecy>
**What it is**: `Secret<T>` wrapper that prevents values from appearing in `Debug` or `Display` output.
**Where you'll see it**: API keys in `ChatOpenAI`, `ChatOllama`, and any credential provider.
**When to use it directly**: Wrap API keys in `Secret<String>` in your own provider or tool implementations. Never store secrets in plain `String` fields.

---

## Persistence

### `rusqlite`

**docs.rs**: <https://docs.rs/rusqlite>
**What it is**: Synchronous SQLite bindings for Rust. Used with the `bundled` feature — no system `libsqlite3` required.
**Where you'll see it**: `synwire-checkpoint-sqlite` uses this for durable checkpoint storage.
**When to use it directly**: When implementing a custom `BaseCheckpointSaver` backed by SQLite.

### `r2d2` + `r2d2_sqlite`

**docs.rs**: <https://docs.rs/r2d2>
**What it is**: Thread-safe connection pool. `r2d2_sqlite` provides the SQLite adapter.
**Where you'll see it**: `SqliteSaver` uses `r2d2` to pool SQLite connections for concurrent checkpoint reads/writes.
**When to use it directly**: Any custom synchronous SQLite-backed component.

---

## Observability

### `tracing`

**docs.rs**: <https://docs.rs/tracing>
**What it is**: Structured, async-aware logging and span tracing. `#[tracing::instrument]` automatically records function arguments and execution time.
**Where you'll see it**: All Synwire async operations emit `tracing` spans when the `tracing` feature is enabled.
**When to use it directly**: Add `#[tracing::instrument]` to your `AgentNode::process` implementations and use `tracing::info!` / `tracing::debug!` for observability. Wire up a subscriber (e.g., `tracing-subscriber`) in your `main`.

### `opentelemetry` + `tracing-opentelemetry`

**docs.rs**: <https://docs.rs/opentelemetry>
**What it is**: OpenTelemetry SDK and the `tracing` bridge that exports spans to OTLP collectors.
**Where you'll see it**: Optional feature for exporting Synwire traces to Jaeger, Honeycomb, etc.
**When to use it directly**: When you need distributed tracing across microservices.

---

## Caching

### `moka`

**docs.rs**: <https://docs.rs/moka>
**What it is**: Async-aware, bounded, concurrent LRU cache.
**Where you'll see it**: `CacheBackedEmbeddings` in the `synwire` umbrella crate caches embedding vectors to avoid redundant API calls.
**When to use it directly**: When building a custom `CachingMiddleware` variant or caching tool results.

---

## Schema generation

### `schemars`

**docs.rs**: <https://docs.rs/schemars>
**What it is**: Derives JSON Schema from Rust types. Used by `#[tool]` to generate tool input schemas.
**Where you'll see it**: Every `#[tool]`-annotated function's parameter type must implement `JsonSchema`.
**When to use it directly**: `#[derive(JsonSchema)]` on your tool parameter structs. Add `#[schemars(description = "...")]` on fields to populate JSON Schema descriptions.

---

## Compression / archive

### `tar` / `flate2` / `zip`

**docs.rs**: <https://docs.rs/tar>, <https://docs.rs/flate2>, <https://docs.rs/zip>
**What they are**: Rust implementations of tar, gzip, and zip archive formats.
**Where you'll see them**: `ArchiveManager` uses all three to read and write archive files.
**When to use them directly**: You interact via `ArchiveManager`, not these crates directly.

---

## Utilities

### `uuid`

**docs.rs**: <https://docs.rs/uuid>
**What it is**: UUID generation (v4 random, v7 sortable).
**Where you'll see it**: Session IDs, checkpoint IDs, job IDs throughout the runtime.
**When to use it directly**: `Uuid::new_v4()` for your own entity IDs.

### `chrono`

**docs.rs**: <https://docs.rs/chrono>
**What it is**: Date and time types with serde support.
**Where you'll see it**: `SessionMetadata.created_at`, `SessionMetadata.updated_at`, timestamps in checkpoint metadata.
**When to use it directly**: `chrono::Utc::now()` for timestamps in custom session or checkpoint implementations.

### `regex`

**docs.rs**: <https://docs.rs/regex>
**What it is**: Regular expression engine (NFA-based, no backtracking, linear time).
**Where you'll see it**: `GrepOptions.pattern` is compiled to a `regex::Regex` inside `LocalProvider` and `Shell`.
**When to use it directly**: Custom search backends.

### `bitflags`

**docs.rs**: <https://docs.rs/bitflags>
**What it is**: `bitflags!` macro for type-safe flag sets backed by an integer.
**Where you'll see it**: Permission flag sets in the agent runtime.
**When to use it directly**: Custom permission or capability flag types.

### `dyn-clone`

**docs.rs**: <https://docs.rs/dyn-clone>
**What it is**: `DynClone` trait that enables `clone()` on trait objects.
**Where you'll see it**: Synwire clones boxed `Middleware` and `Plugin` instances when building runner variants.
**When to use it directly**: When implementing `Middleware` or `Plugin` and your type needs to be cloneable behind a `Box<dyn ...>`.

---

## Protocol integration

### `async-lsp`

**docs.rs**: <https://docs.rs/async-lsp>
**What it is**: Tower-based async Language Server Protocol client and server library. Handles `Content-Length` framing over stdio, request/response correlation, and middleware (concurrency limits, tracing, panic catching).
**Where you'll see it**: `synwire-lsp` uses `MainLoop::new_client()` and `ServerSocket` to communicate with language servers.
**When to use it directly**: Only via `synwire-lsp` — the crate wraps `async-lsp` and exposes agent-friendly tools.

### `lsp-types`

**docs.rs**: <https://docs.rs/lsp-types>
**What it is**: Complete Rust type definitions for the Language Server Protocol specification. Approximately 300 types covering all LSP requests, responses, notifications, and data structures.
**Where you'll see it**: All LSP request/response types in `synwire-lsp` (`GotoDefinitionResponse`, `Hover`, `CompletionResponse`, `ServerCapabilities`, etc.).
**When to use it directly**: When constructing custom LSP request parameters or interpreting tool outputs.

### `dapts`

**docs.rs**: <https://docs.rs/dapts>
**What it is**: Auto-generated Rust type definitions for the Debug Adapter Protocol specification. Covers requests, responses, events, and data types.
**Where you'll see it**: DAP message types in `synwire-dap`.
**When to use it directly**: When constructing custom DAP request parameters or interpreting debug tool outputs.

---

## Testing

### `mockall`

**docs.rs**: <https://docs.rs/mockall>
**What it is**: `#[automock]` attribute that generates a full mock struct for any trait, with call count assertions, argument matchers, and sequence enforcement.
**Where you'll see it**: Internal Synwire unit tests; you can use it in your own tests.
**When to use it directly**: When `FakeChatModel` isn't expressive enough — e.g., you need to assert "this method was called exactly 3 times with argument X".

### `proptest`

**docs.rs**: <https://docs.rs/proptest>
**What it is**: Property-based testing framework. Generates random inputs from strategies and shrinks failures.
**Where you'll see it**: `synwire-test-utils::strategies` exposes proptest strategies for all Synwire types.
**When to use it directly**: `proptest! { #[test] fn ... }` macro for property tests. Use the strategies from `synwire-test-utils` rather than writing your own.

### `tokio-test`

**docs.rs**: <https://docs.rs/tokio-test>
**What it is**: Test utilities for async code: `task::spawn`, `assert_ready!`, `assert_pending!`, `io::Builder` for mock I/O.
**Where you'll see it**: Synwire internal tests for async state machines.
**When to use it directly**: When testing custom `Future` or `Stream` implementations.

### `criterion`

**docs.rs**: <https://docs.rs/criterion>
**What it is**: Statistical benchmark harness. Runs benchmarks many times, applies Welch's t-test, and produces HTML reports.
**Where you'll see it**: `benches/` directory in `synwire-core` and `synwire-orchestrator`.
**When to use it directly**: Add a `[[bench]]` section to `Cargo.toml` and write benchmarks with `criterion_group!` and `criterion_main!`.

---

## Proc-macro internals

### `syn` / `quote` / `proc-macro2`

**docs.rs**: <https://docs.rs/syn>, <https://docs.rs/quote>, <https://docs.rs/proc-macro2>
**What they are**: The standard toolkit for writing Rust procedural macros. `syn` parses Rust source into an AST; `quote!` emits new Rust code; `proc-macro2` provides token streams.
**Where you'll see them**: `synwire-derive` uses all three to implement `#[tool]` and `#[derive(State)]`.
**When to use them directly**: When writing your own proc-macros. You do not need these when *using* `synwire-derive`.

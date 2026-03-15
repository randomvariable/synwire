# Feature Flags

## synwire-core

| Feature | Default | Description |
|---------|---------|-------------|
| `retry` | Yes | Retry support via `backoff` + `tokio` |
| `http` | Yes | HTTP client via `reqwest` |
| `tracing` | No | OpenTelemetry tracing integration |
| `event-bus` | No | Tokio-based event bus for custom events |
| `batch-api` | No | Provider-level batch processing trait |

### Example

```toml
[dependencies]
synwire-core = { version = "0.1", features = ["tracing"] }
```

To disable defaults:

```toml
synwire-core = { version = "0.1", default-features = false, features = ["http"] }
```

## synwire (umbrella)

| Feature | Default | Description |
|---------|---------|-------------|
| `openai` | No | Include `synwire-llm-openai` provider |
| `ollama` | No | Include `synwire-llm-ollama` provider |

### Example

```toml
[dependencies]
synwire = { version = "0.1", features = ["openai", "ollama"] }
```

## Provider crates

`synwire-llm-openai` and `synwire-llm-ollama` have no optional features. They always depend on `synwire-core` with the `http` feature enabled.

## synwire-checkpoint-sqlite

No optional features. Always depends on `rusqlite` with the `bundled` feature (compiles SQLite from source).

## synwire-derive

No optional features. Proc-macro crate depending on `syn`, `quote`, `proc-macro2`.

## Interaction between features

- `retry` requires `tokio` for async backoff delays
- `tracing` enables `tracing`, `tracing-opentelemetry`, `opentelemetry`, and `opentelemetry_sdk`
- `event-bus` requires `tokio` for broadcast channels
- Disabling `http` removes `reqwest` -- provider crates will not compile without it

## Checking active features

```sh
cargo tree -e features -p synwire-core
```

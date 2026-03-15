# Enable Tracing

Synwire supports OpenTelemetry-based tracing via the `tracing` feature flag on `synwire-core`.

## Enable the feature

```toml
[dependencies]
synwire-core = { version = "0.1", features = ["tracing"] }
```

## Setup tracing subscriber

```rust,ignore
use tracing_subscriber::prelude::*;

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
}
```

## Callbacks for custom observability

Implement `CallbackHandler` to receive events during execution:

```rust,ignore
use synwire_core::callbacks::CallbackHandler;
use synwire_core::BoxFuture;

struct MetricsCallback;

impl CallbackHandler for MetricsCallback {
    fn on_llm_start<'a>(
        &'a self,
        model_type: &'a str,
        messages: &'a [synwire_core::messages::Message],
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            // Record metrics, emit spans, etc.
        })
    }

    fn on_llm_end<'a>(
        &'a self,
        response: &'a serde_json::Value,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            // Record latency, token usage, etc.
        })
    }
}
```

## Attaching callbacks to invocations

Pass callbacks via `RunnableConfig`:

```rust,ignore
use synwire_core::runnables::RunnableConfig;

let config = RunnableConfig {
    callbacks: vec![Box::new(MetricsCallback)],
    ..Default::default()
};

let result = model.invoke(&messages, Some(&config)).await?;
```

## Filtering callback events

Override the `ignore_*` methods to skip categories:

```rust,ignore
impl CallbackHandler for MetricsCallback {
    fn ignore_tool(&self) -> bool { true }  // Skip tool events
    fn ignore_llm(&self) -> bool { false }  // Keep LLM events
}
```

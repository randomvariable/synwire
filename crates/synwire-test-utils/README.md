# synwire-test-utils

Test helpers for Synwire applications. Fake models, recording executors, proptest strategies, fixture builders, and backend conformance suites.

> **Important**: Add this crate to `[dev-dependencies]` only — never `[dependencies]`.

## What this crate provides

- **`FakeChatModel`** — pre-programmed responses; deterministic; no network; error injection
- **`FakeEmbeddings`** — returns zero-vectors or fixed vectors; configurable dimension
- **`RecordingExecutor`** — captures `Directive` values without executing them
- **Proptest strategies** — `arb_tool_schema()`, `arb_message()`, `arb_directive()`, `arb_document()`, `arb_checkpoint()`, and more
- **Fixture builders** — `DocumentBuilder`, `MessageBuilder`, `PromptTemplateBuilder`, `ToolSchemaBuilder`
- **Conformance suite** — `conformance::run_backend_conformance`, `conformance::run_session_conformance`
- **`executors` module** — test executor utilities

## Quick start

```toml
[dev-dependencies]
synwire-test-utils = "0.1"
tokio = { version = "1", features = ["full"] }
```

Use `FakeChatModel` for a deterministic test:

```rust,no_run
#[cfg(test)]
mod tests {
    use synwire_test_utils::FakeChatModel;
    use synwire_core::language_models::chat::BaseChatModel;

    #[tokio::test]
    async fn model_returns_greeting() {
        let model = FakeChatModel::new(vec!["Hello!".to_string()]);
        let result = model.invoke("Hi").await.unwrap();
        assert_eq!(result.content, "Hello!");
    }
}
```

Use `RecordingExecutor` to assert agent intent without executing effects:

```rust,no_run
#[cfg(test)]
mod tests {
    use synwire_test_utils::executors::RecordingExecutor;
    use synwire_core::agents::directive::Directive;

    #[tokio::test]
    async fn agent_emits_stop_on_completion() {
        let executor = RecordingExecutor::new();
        // run your AgentNode against the executor...
        // let directives = executor.recorded();
        // assert!(directives.iter().any(|d| matches!(d, Directive::Stop { .. })));
    }
}
```

Property-based testing with proptest:

```rust,no_run
use synwire_test_utils::strategies::arb_message;
use proptest::prelude::*;

proptest! {
    #[test]
    fn message_round_trips_json(msg in arb_message()) {
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: synwire_core::messages::Message = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(msg, decoded);
    }
}
```

Backend conformance:

```rust,no_run
#[tokio::test]
async fn my_backend_passes_conformance() {
    let backend = MyCustomBackend::new();
    synwire_test_utils::conformance::run_backend_conformance(backend).await;
}
```

## Documentation

- [Testing Without Side Effects](https://randomvariable.github.io/synwire/tutorials/02-pure-directive-testing.html)
- [Testing Explanation](https://randomvariable.github.io/synwire/explanation/synwire-test-utils.html)
- [Full API docs](https://docs.rs/synwire-test-utils)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

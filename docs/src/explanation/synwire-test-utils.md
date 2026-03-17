# synwire-test-utils: Testing Synwire Applications

`synwire-test-utils` provides everything you need to test Synwire-based code without network access, real LLM costs, or side effects.

> **Important**: Always add this crate to `[dev-dependencies]`, never `[dependencies]`.

## `FakeChatModel`: deterministic responses

The simplest way to test any code that takes a `BaseChatModel`:

```rust,no_run
#[cfg(test)]
mod tests {
    use synwire_test_utils::FakeChatModel;
    use synwire_core::language_models::chat::BaseChatModel;

    #[tokio::test]
    async fn greeting_pipeline_formats_response() {
        // Responses are returned in order; last one repeats if the list is exhausted
        let model = FakeChatModel::new(vec![
            "Hello, Alice!".to_string(),
            "Hello, Bob!".to_string(),
        ]);

        let r1 = model.invoke("greet Alice").await.unwrap();
        assert_eq!(r1.content, "Hello, Alice!");

        let r2 = model.invoke("greet Bob").await.unwrap();
        assert_eq!(r2.content, "Hello, Bob!");
    }
}
```

Inject errors at specific positions:

```rust,no_run
use synwire_test_utils::FakeChatModel;
use synwire_core::SynwireError;

let model = FakeChatModel::new(vec!["ok".to_string()])
    .with_error_at(0, SynwireError::RateLimit("retry after 1s".to_string()));
// model.invoke(...) on call 0 → Err(RateLimit)
```

## `RecordingExecutor`: assert agent intent

`RecordingExecutor` captures `Directive` values without executing them. This is the canonical way to test the [directive/effect pattern](./agent-core-directive-effect-architecture.md):

```rust,no_run
#[cfg(test)]
mod tests {
    use synwire_test_utils::executors::RecordingExecutor;
    use synwire_core::agents::directive::Directive;

    #[tokio::test]
    async fn planner_node_emits_run_instruction() {
        let executor = RecordingExecutor::new();
        // run your AgentNode::process with executor as the DirectiveExecutor
        // ...
        let directives = executor.recorded();
        assert!(
            directives.iter().any(|d| matches!(d, Directive::RunInstruction { .. })),
            "planner must emit at least one RunInstruction"
        );
    }
}
```

This test validates *intent* (the directive type emitted) without triggering filesystem operations, HTTP calls, or any side effect. Tests remain deterministic regardless of environment.

## Proptest strategies

Write property-based tests over Synwire types:

> 📖 **Rust note:** [Property-based testing](https://proptest-rs.github.io/proptest/intro.html) generates random inputs from a *strategy* and checks that a property holds for all of them. `proptest!` is a macro that runs many randomised trials automatically.

```rust,no_run
use synwire_test_utils::strategies::arb_message;
use proptest::prelude::*;

proptest! {
    #[test]
    fn message_serialises_and_deserialises(msg in arb_message()) {
        let json = serde_json::to_string(&msg).unwrap();
        let recovered: synwire_core::messages::Message = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(msg.role, recovered.role);
    }
}
```

Available strategies (all in `synwire_test_utils::strategies`):

| Strategy | Produces |
|---|---|
| `arb_message()` | `Message` |
| `arb_tool_schema()` | `ToolSchema` |
| `arb_directive()` | `Directive` |
| `arb_document()` | `Document` |
| `arb_checkpoint()` | `Checkpoint` |

## Fixture builders

Concisely construct test data without filling in every field:

```rust,no_run
use synwire_test_utils::builders::{MessageBuilder, DocumentBuilder};

let msg = MessageBuilder::new()
    .role("user")
    .content("What is ownership?")
    .build();

let doc = DocumentBuilder::new()
    .content("Ownership is Rust's memory management strategy.")
    .metadata("source", "rust-book")
    .build();
```

## Backend conformance suite

If you implement `Vfs`, run the conformance suite to verify correctness:

```rust,no_run
#[tokio::test]
async fn my_backend_satisfies_contract() {
    let backend = MyCustomBackend::new("/tmp/test-root");
    synwire_test_utils::conformance::run_vfs_conformance(backend).await;
}

#[tokio::test]
async fn my_session_manager_satisfies_contract() {
    let mgr = MySessionManager::new();
    synwire_test_utils::conformance::run_session_conformance(mgr).await;
}
```

The conformance suite exercises the full `Vfs` / `SessionManager` API surface and asserts correctness at each step.

## When to use `mockall` instead

Use `mockall`'s `#[automock]` when you need:
- **Call count assertions** — "this method must be called exactly twice"
- **Argument matching** — "the second argument must equal X"
- **Complex call sequences** — ordering guarantees across multiple methods

`FakeChatModel` is simpler but less powerful than a full mock. For most agent tests, `FakeChatModel` + `RecordingExecutor` is sufficient.

## See also

- [Testing Without Side Effects](../tutorials/02-pure-directive-testing.md)
- [Directive/Effect Architecture](./agent-core-directive-effect-architecture.md)
- [Contributing Style Guide](../contributing/style-guide.md)

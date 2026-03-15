# Migration from LangChain Python

This guide maps LangChain Python concepts to their Synwire Rust equivalents.

## Naming

| Python | Rust |
|--------|------|
| `langchain` | `synwire` (umbrella crate) |
| `langchain-core` | `synwire-core` |
| `langgraph` | `synwire-orchestrator` |
| `langchain-openai` | `synwire-llm-openai` |

## Core concepts

| Python concept | Rust equivalent | Notes |
|---------------|-----------------|-------|
| `BaseChatModel` | `BaseChatModel` trait | Same name, trait-based |
| `Embeddings` | `Embeddings` trait | Same interface |
| `VectorStore` | `VectorStore` trait | Same interface |
| `Tool` / `@tool` | `Tool` trait / `#[tool]` | Macro replaces decorator |
| `Runnable` | `RunnableCore` trait | Uses `serde_json::Value` I/O |
| `RunnableSequence` | `RunnableSequence` / `pipe()` | Explicit composition |
| `OutputParser` | `OutputParser` trait | Associated `Output` type |
| `CallbackHandler` | `CallbackHandler` trait | Async methods |
| `StateGraph` | `StateGraph` | Same API surface |

## Key differences

### Async is explicit

Python:
```python
result = await model.ainvoke(messages)
```

Rust:
```rust,ignore
let result = model.invoke(&messages, None).await?;
```

There is no sync `invoke` -- all I/O is async. Use `tokio::runtime::Runtime::block_on` if you need sync access.

### Error handling

Python uses exceptions. Rust uses `Result<T, SynwireError>` with the `?` operator:

```rust,ignore
let result = model.invoke(&messages, None).await?;
let output = parser.parse(&result.message.content().as_text())?;
```

### No dynamic typing

Python's duck typing becomes explicit traits:

```python
# Python: any object with .invoke() works
chain = prompt | model | parser
```

```rust,ignore
// Rust: types must implement RunnableCore
let sequence = pipe(vec![
    Box::new(prompt_runnable),
    Box::new(model_runnable),
    Box::new(parser_runnable),
]);
```

### Ownership and borrowing

Where Python passes objects freely, Rust requires explicit ownership decisions:

- `Box<dyn BaseChatModel>` -- owned, single owner
- `Arc<dyn BaseChatModel>` -- shared ownership across tasks
- `&dyn BaseChatModel` -- borrowed reference

### Messages are enums, not classes

```rust,ignore
// Python: HumanMessage("Hello")
// Rust:
let msg = Message::human("Hello");
```

### Configuration is per-call

Python uses class-level configuration. Rust passes `RunnableConfig` per invocation:

```rust,ignore
let config = RunnableConfig { /* callbacks, tags, metadata */ ..Default::default() };
let result = model.invoke(&messages, Some(&config)).await?;
```

## Feature flags replace optional dependencies

Python:
```bash
pip install langchain-openai
```

Rust:
```toml
synwire = { version = "0.1", features = ["openai"] }
# or directly:
synwire-llm-openai = "0.1"
```

## Testing

| Python | Rust |
|--------|------|
| `FakeChatModel` | `FakeChatModel` |
| `FakeEmbeddings` | `FakeEmbeddings` |
| `pytest` | `cargo nextest` / `cargo test` |
| `unittest.mock` | `mockall` crate |
| Property testing | `proptest` crate |

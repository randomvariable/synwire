# synwire-core: The Trait Contract Layer

`synwire-core` is Synwire's foundational crate. It defines every public trait and shared type but ships zero concrete implementations. If you are writing something that others plug into Synwire, this is the only crate you need.

> 📖 **Rust note:** A [trait](https://doc.rust-lang.org/book/ch10-02-traits.html) is Rust's equivalent of an interface — it defines a set of methods a type must implement. `impl Trait` in a function argument accepts any type that satisfies it; `dyn Trait` allows holding different implementing types behind a pointer at runtime.

## When to depend on `synwire-core` directly

Depend on `synwire-core` (not on `synwire` or provider crates) when:

- You are **publishing a third-party extension crate** — a custom `VectorStore`, a bespoke `SessionManager`, a new LLM provider — and you want users to be able to depend on your crate without pulling in `synwire-agent` or any provider
- You want to **write application code that is provider-agnostic** — store the model as `Box<dyn BaseChatModel>` and swap implementations in tests vs production
- You are writing **integration tests** that should compile without any concrete provider dependencies

If you are building an **end-user application**, use `synwire` (the umbrella crate with re-exports) or a provider crate directly — you rarely need to import `synwire-core` explicitly.

## Trait hierarchy

```rust,no_run
use synwire_core::language_models::chat::BaseChatModel;
use synwire_core::embeddings::Embeddings;
use synwire_core::tools::Tool;
use synwire_core::agents::{AgentNode, ExecutionStrategy, Vfs, Middleware, Plugin, SessionManager};

// Store any chat model implementation behind a trait object:
fn build_pipeline(model: Box<dyn BaseChatModel>) {
    // model.invoke(...), model.stream(...), model.bind_tools(...)
}
```

| Trait | Purpose | Implemented in |
|---|---|---|
| `BaseChatModel` | Chat completions: `invoke`, `batch`, `stream`, `model_type`, `bind_tools` | `synwire-llm-openai`, `synwire-llm-ollama`, `FakeChatModel` |
| `BaseLLM` | Text-completion (string in, string out) | Provider crates |
| `Embeddings` | `embed_documents`, `embed_query` | `synwire-llm-openai`, `synwire-llm-ollama` |
| `VectorStore` | Document storage with similarity search | User-implemented or third-party |
| `Tool` / `StructuredTool` | Callable tools with JSON Schema | Any `#[tool]` function, user-implemented |
| `RunnableCore` | Universal composition via `serde_json::Value` I/O | All runnables |
| `OutputParser` | Typed output parsing from model responses | `synwire` umbrella crate |
| `DocumentLoader` | Async document ingestion | User-implemented or third-party |
| `AgentNode` | Agent turn logic returning `DirectiveResult` | User-implemented |
| `ExecutionStrategy` | Controls how the runner sequences turns | `DirectStrategy`, `FsmStrategy` in `synwire-agent` |
| `Vfs` | File, shell, HTTP, and process operations as effects | All backends in `synwire-agent` |
| `Middleware` | Applied before each agent turn | All middleware in `synwire-agent` |
| `Plugin` | Stateful component with lifecycle hooks | User-implemented |
| `SessionManager` | Session CRUD: create, get, update, delete, list, fork, rewind, tag | `InMemorySessionManager` in `synwire-agent` |
| `McpTransport` | MCP protocol transport | stdio/HTTP/in-process variants in `synwire-agent` |

## Key shared types

- **`BoxFuture<'a, T>`** — `Pin<Box<dyn Future<Output = T> + Send + 'a>>`; used by all async trait methods

> 📖 **Rust note:** `BoxFuture<'_, T>` is shorthand for `Pin<Box<dyn Future<Output = T> + Send + '_>>`. Trait methods can't use `async fn` directly and remain [object-safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety), so Synwire returns heap-allocated, pinned futures instead. You interact with these via `.await` exactly like any other future.

- **`BoxStream<'a, T>`** — `Pin<Box<dyn Stream<Item = T> + Send + 'a>>`; used by streaming methods
- **`Message` / `MessageContent` / `ContentBlock`** — chat message types
- **`ChatResult` / `ChatChunk`** — invoke and stream response types
- **`ToolSchema` / `ToolOutput` / `ToolCall`** — tool interface types
- **`Document`** — a text chunk with metadata, used by loaders and vector stores
- **`SynwireError`** — top-level library error; all public APIs return `Result<T, SynwireError>`
- **`Directive`** — an intended effect returned by `AgentNode::process`
- **`DirectiveResult<S>`** — `Result<AgentEvent, AgentError>`

## Implementing a custom `BaseChatModel`

```rust,no_run
use synwire_core::language_models::chat::{BaseChatModel, ChatResult, ChatChunk};
use synwire_core::{BoxFuture, BoxStream, SynwireError};

struct MyModel;

impl BaseChatModel for MyModel {
    fn model_type(&self) -> &str { "my-model" }

    fn invoke<'a>(&'a self, input: &'a str) -> BoxFuture<'a, Result<ChatResult, SynwireError>> {
        Box::pin(async move {
            Ok(ChatResult { content: format!("Echo: {input}"), ..Default::default() })
        })
    }

    fn stream<'a>(&'a self, input: &'a str) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, SynwireError>>, SynwireError>> {
        // ... stream implementation
        todo!()
    }
}
```

## Feature flags

| Flag | Enables |
|---|---|
| `retry` | `reqwest-retry` middleware for automatic retries on transient HTTP errors |
| `http` | `reqwest` HTTP client (needed by provider crates) |
| `tracing` | `tracing` spans on all async operations |
| `event-bus` | Internal event bus for cross-component messaging |
| `batch-api` | Batch request support for providers that offer it |

## See also

- [Architecture](./architecture.md) — trait hierarchy in context
- [Crate Architecture and Layer Boundaries](./agent-core-crate-structure.md) — where each trait lives
- [Agent Runtime (`synwire-agent`)](./synwire-agent.md) — concrete implementations of these traits

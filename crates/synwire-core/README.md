# synwire-core

The foundational trait layer for the Synwire AI framework. Contains every public trait and shared type; zero concrete implementations, zero `unsafe` code.

## What this crate provides

- **`BaseChatModel`** — `invoke`, `batch`, `stream`, `model_type`, `bind_tools`; the primary interface for all chat LLMs
- **`BaseLLM`** — text-completion (single string in, single string out)
- **`Embeddings`** — `embed_documents`, `embed_query`; implemented by OpenAI and Ollama providers
- **`VectorStore`** — document storage with similarity search
- **`Tool` / `StructuredTool`** — callable tools with JSON Schema input validation
- **`RunnableCore`** — universal composition primitive using `serde_json::Value` I/O
- **`OutputParser`** — typed output parsing from model responses
- **`DocumentLoader`** — async document ingestion
- **`AgentNode`** — agent turn logic returning `DirectiveResult`
- **`ExecutionStrategy`** — controls how the runner sequences agent turns
- **`BackendProtocol`** — file, shell, HTTP, and process operations as algebraic effects
- **`Middleware`** — applied before each agent turn to augment context
- **`Plugin`** — stateful component with lifecycle hooks
- **`SessionManager`** — session CRUD: create, get, update, delete, list, fork, rewind, tag
- **`McpTransport`** — MCP protocol transport (stdio, HTTP, in-process)

## Quick start

```toml
[dependencies]
synwire-core = "0.1"

[dev-dependencies]
synwire-test-utils = "0.1"
```

Invoke a `FakeChatModel` (no network required):

```rust,no_run
use synwire_core::language_models::chat::BaseChatModel;
use synwire_test_utils::FakeChatModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = FakeChatModel::new(vec!["Hello from Synwire!".to_string()]);
    let result = model.invoke("Hi").await?;
    println!("{}", result.content);
    Ok(())
}
```

Implement a custom `Tool`:

```rust,no_run
use synwire_core::tools::{Tool, ToolOutput, ToolSchema};
use synwire_core::error::SynwireError;
use synwire_core::BoxFuture;

struct EchoTool {
    schema: ToolSchema,
}

impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Returns its input unchanged" }
    fn schema(&self) -> &ToolSchema { &self.schema }

    fn invoke(&self, input: serde_json::Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            Ok(ToolOutput {
                content: input.to_string(),
                ..Default::default()
            })
        })
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
| `batch-api` | Batch request support for LLM providers that offer it |

## Crates that implement these traits

| Crate | Implements |
|---|---|
| `synwire-llm-openai` | `BaseChatModel`, `Embeddings` |
| `synwire-llm-ollama` | `BaseChatModel`, `Embeddings` |
| `synwire-agent` | `ExecutionStrategy`, `Vfs`, `Middleware`, `SessionManager`, `McpTransport` |
| `synwire-checkpoint` | `BaseCheckpointSaver`, `BaseStore` |
| `synwire-test-utils` | `BaseChatModel` (`FakeChatModel`), `Embeddings` (`FakeEmbeddings`) |

## Documentation

- [Architecture Explanation](https://randomvariable.github.io/synwire/explanation/synwire-core.html)
- [Full API docs](https://docs.rs/synwire-core)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

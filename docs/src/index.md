# Synwire

Synwire is an async-first Rust framework for building LLM-powered applications, designed around trait-based composition and zero `unsafe` code.

## Key features

- **Trait-based architecture** -- swap providers, vector stores, and tools via trait objects
- **Async-first** -- all I/O uses `async`/`await` with Tokio
- **Graph orchestration** -- build stateful agent workflows with `StateGraph`
- **Type-safe macros** -- `#[tool]` and `#[derive(State)]` for ergonomic definitions
- **Comprehensive testing** -- `FakeChatModel` and `FakeEmbeddings` for offline testing
- **Zero unsafe code** -- `#![forbid(unsafe_code)]` in core crates

## Quick example

```rust,ignore
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_core::messages::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = FakeChatModel::new(vec!["Hello from Synwire!".into()]);
    let messages = vec![Message::human("Hi")];
    let result = model.invoke(&messages, None).await?;
    println!("{}", result.message.content().as_text());
    Ok(())
}
```

## Crate overview

| Crate | Description |
|-------|-------------|
| `synwire-core` | Core traits: `BaseChatModel`, `Embeddings`, `VectorStore`, `Tool`, `RunnableCore` |
| `synwire-orchestrator` | Graph-based orchestration: `StateGraph`, `CompiledGraph`, channels |
| `synwire-checkpoint` | Checkpoint traits and in-memory implementation |
| `synwire-checkpoint-sqlite` | SQLite checkpoint backend |
| `synwire-llm-openai` | OpenAI provider (`ChatOpenAI`, `OpenAIEmbeddings`) |
| `synwire-llm-ollama` | Ollama provider (`ChatOllama`, `OllamaEmbeddings`) |
| `synwire-derive` | Procedural macros (`#[tool]`, `#[derive(State)]`) |
| `synwire-test-utils` | Fake models, proptest strategies, fixture builders |
| `synwire` | Convenience re-exports, caches, text splitters, few-shot prompts |

## Navigation

- **Getting Started** -- step-by-step tutorials from first chat to graph agents
- **How-To Guides** -- task-focused recipes for common operations
- **Explanation** -- design rationale and architecture decisions
- **Reference** -- glossary, error guide, feature flags
- **Contributing** -- setup, style guide

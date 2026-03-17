# synwire

Convenience umbrella crate for the Synwire AI framework. One dependency gives you chat models, embeddings, graph orchestration, agent runtime, and utilities.

## What this crate provides

Re-exports from all Synwire crates plus:

- **`agent::prelude::*`** — `Agent`, `AgentNode`, `Runner`, `Directive`, `DirectiveResult`, `AgentError`, `AgentEvent`, `Session`, `SessionManager`, `Usage`, `OutputMode`, `HookRegistry`
- **`cache::CacheBackedEmbeddings`** — moka-backed embedding cache wrapping any `Embeddings` impl
- **`chat_history::InMemoryChatMessageHistory`** + **`RunnableWithMessageHistory`** — stateful conversation management
- **`prompts`** — few-shot prompt templates, example selectors
- **`text_splitters`** — `RecursiveCharacterTextSplitter`, character splitter
- **`output_parsers`** — `CommaSeparatedListOutputParser`, `RetryOutputParser`, `RegexParser`, `EnumOutputParser`, `XmlOutputParser`, `CombiningOutputParser`

## Quick start

```toml
[dependencies]
synwire = { version = "0.1", features = ["openai"] }
tokio = { version = "1", features = ["full"] }
```

First chat:

```rust,no_run
use synwire::prelude::*;
use synwire::llm_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    let result = model.invoke("Hello, Synwire!").await?;
    println!("{}", result.content);
    Ok(())
}
```

Cached embeddings:

```rust,no_run
use synwire::cache::CacheBackedEmbeddings;
use synwire::llm_openai::OpenAIEmbeddings;

let base = OpenAIEmbeddings::builder().api_key_env("OPENAI_API_KEY").build()?;
let cached = CacheBackedEmbeddings::new(base, 1000); // LRU cache, 1000 entries
```

Stateful conversation:

```rust,no_run
use synwire::chat_history::{InMemoryChatMessageHistory, RunnableWithMessageHistory};
use synwire::llm_openai::ChatOpenAI;

let model = ChatOpenAI::builder().model("gpt-4o").api_key_env("OPENAI_API_KEY").build()?;
let history = InMemoryChatMessageHistory::new();
let chain = RunnableWithMessageHistory::new(model, history);
```

## Feature flags

| Flag | Enables |
|---|---|
| `openai` | `synwire-llm-openai` (ChatOpenAI, OpenAIEmbeddings) |
| `ollama` | `synwire-llm-ollama` (ChatOllama, OllamaEmbeddings) |

## When to use this crate vs individual crates

- **Use `synwire`** for application development and prototypes
- **Use `synwire-core` directly** when publishing a third-party extension crate — it carries no provider dependencies

## Documentation

- [Getting Started](https://randomvariable.github.io/synwire/getting-started/first-chat.html)
- [Full API docs](https://docs.rs/synwire)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

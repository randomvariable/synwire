# synwire-llm-ollama

Local LLM inference for Synwire via [Ollama](https://ollama.com). No API key required — all inference runs on your machine.

## What this crate provides

- **`ChatOllama`** — chat completions using any Ollama-served model
- **`OllamaEmbeddings`** — local text embeddings (nomic-embed-text, mxbai-embed-large, etc.)
- Both implement `BaseChatModel` / `Embeddings` — swap with OpenAI by changing one line

## Prerequisites

```sh
# Install Ollama (https://ollama.com)
ollama pull llama3.2
```

## Quick start

```toml
[dependencies]
synwire-llm-ollama = "0.1"
tokio = { version = "1", features = ["full"] }
```

```rust,no_run
use synwire_llm_ollama::ChatOllama;
use synwire_core::language_models::chat::BaseChatModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOllama::builder()
        .model("llama3.2")
        .build()?;

    let result = model.invoke("What is the Rust borrow checker?").await?;
    println!("{}", result.content);
    Ok(())
}
```

Streaming:

```rust,no_run
use synwire_llm_ollama::ChatOllama;
use synwire_core::language_models::chat::BaseChatModel;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOllama::builder().model("llama3.2").build()?;
    let mut stream = model.stream("Count to five.").await?;
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?.content);
    }
    Ok(())
}
```

Local RAG with embeddings:

```rust,no_run
use synwire_llm_ollama::OllamaEmbeddings;
use synwire_core::embeddings::Embeddings;

let embeddings = OllamaEmbeddings::builder()
    .model("nomic-embed-text")
    .build()?;

let vectors = embeddings.embed_query("semantic search query").await?;
```

Swap from OpenAI to Ollama — one line changes:

```rust,no_run
use synwire_core::language_models::chat::BaseChatModel;

// Before:
// let model: Box<dyn BaseChatModel> = Box::new(ChatOpenAI::builder().model("gpt-4o").build()?);
// After:
// let model: Box<dyn BaseChatModel> = Box::new(ChatOllama::builder().model("llama3.2").build()?);
// The rest of your code is unchanged.
```

## Documentation

- [Local Inference with Ollama](https://randomvariable.github.io/synwire/getting-started/ollama.html)
- [LLM Providers Explanation](https://randomvariable.github.io/synwire/explanation/synwire-llm-providers.html)
- [Full API docs](https://docs.rs/synwire-llm-ollama)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

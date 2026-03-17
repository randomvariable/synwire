# Local Inference with Ollama

Ollama lets you run large language models on your own machine — no API key, no data leaving the network boundary. `synwire-llm-ollama` implements the same `BaseChatModel` and `Embeddings` traits as the OpenAI provider, so switching is a one-line change.

**When to use Ollama:**
- Privacy-sensitive workloads (data must not leave the machine)
- Air-gapped environments
- Development and testing without API costs
- Experimenting with open-weight models (Llama 3, Mistral, Gemma, Phi)

> 📖 **Rust note:** A [trait](https://doc.rust-lang.org/book/ch10-02-traits.html) is Rust's equivalent of an interface. `BaseChatModel` is a trait — because both `ChatOllama` and `ChatOpenAI` implement it, you can store either behind a `Box<dyn BaseChatModel>` and swap them without changing any other code.

## Prerequisites

1. Install Ollama from <https://ollama.com>
2. Pull a model:

```sh
ollama pull llama3.2
```

3. Confirm it is running:

```sh
ollama run llama3.2 "hello"
```

Ollama listens on `http://localhost:11434` by default.

## Add the dependency

```toml
[dependencies]
synwire-llm-ollama = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Basic invoke

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

## Streaming

> 📖 **Rust note:** [`async fn`](https://doc.rust-lang.org/book/ch17-00-async-await.html) and `.await` let this code run concurrently without blocking a thread. `StreamExt::next().await` yields each chunk as the model generates it — you see output appear progressively rather than waiting for the full response.

```rust,no_run
use synwire_llm_ollama::ChatOllama;
use synwire_core::language_models::chat::BaseChatModel;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOllama::builder()
        .model("llama3.2")
        .build()?;

    let mut stream = model.stream("Explain ownership in Rust step by step.").await?;
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?.content);
    }
    println!();
    Ok(())
}
```

## Local RAG with `OllamaEmbeddings`

Use a local embedding model so that retrieval-augmented generation never sends data to an external API:

```sh
ollama pull nomic-embed-text
```

```rust,no_run
use synwire_llm_ollama::{ChatOllama, OllamaEmbeddings};
use synwire_core::embeddings::Embeddings;
use synwire_core::language_models::chat::BaseChatModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let embeddings = OllamaEmbeddings::builder()
        .model("nomic-embed-text")
        .build()?;

    // Embed your documents
    let docs = vec![
        "Rust ownership means each value has exactly one owner.".to_string(),
        "The borrow checker enforces ownership rules at compile time.".to_string(),
    ];
    let vectors = embeddings.embed_documents(docs).await?;
    println!("Embedded {} documents, dimension {}", vectors.len(), vectors[0].len());

    // Embed a query
    let query_vec = embeddings.embed_query("what is ownership?").await?;
    println!("Query vector dimension: {}", query_vec.len());

    // Use vectors with your vector store, then answer with the chat model:
    let model = ChatOllama::builder().model("llama3.2").build()?;
    let answer = model.invoke("Given context about Rust ownership, explain it simply.").await?;
    println!("{}", answer.content);

    Ok(())
}
```

See [Getting Started: RAG](./rag.md) for a complete retrieval-augmented generation example.

## Swapping from OpenAI to Ollama

Store the model as `Box<dyn BaseChatModel>` — swap by changing the constructor:

```rust,no_run
use synwire_core::language_models::chat::BaseChatModel;

fn build_model() -> anyhow::Result<Box<dyn BaseChatModel>> {
    if std::env::var("USE_LOCAL").is_ok() {
        // Local: no API key required
        Ok(Box::new(
            synwire_llm_ollama::ChatOllama::builder().model("llama3.2").build()?
        ))
    } else {
        // Cloud: reads OPENAI_API_KEY from environment
        Ok(Box::new(
            synwire_llm_openai::ChatOpenAI::builder()
                .model("gpt-4o")
                .api_key_env("OPENAI_API_KEY")
                .build()?
        ))
    }
}
```

All downstream code that calls `model.invoke(...)` or `model.stream(...)` is unchanged.

## Builder options

| Method | Default | Description |
|---|---|---|
| `.model(name)` | — | Required. Any model pulled via `ollama pull` |
| `.base_url(url)` | `http://localhost:11434` | Ollama server address |
| `.temperature(f32)` | model default | Sampling temperature |
| `.top_k(u32)` | model default | Top-k sampling |
| `.top_p(f32)` | model default | Top-p (nucleus) sampling |
| `.num_predict(i32)` | model default | Max tokens to generate (-1 for unlimited) |
| `.timeout(Duration)` | 5 minutes | Request timeout |

## See also

- [Getting Started: First Chat](./first-chat.md) — the same example using OpenAI
- [Getting Started: RAG](./rag.md) — full retrieval pipeline (add `OllamaEmbeddings` for local RAG)
- [LLM Providers Explanation](../explanation/synwire-llm-providers.md) — choosing and swapping providers
- [How-To: Switch Provider](../how-to/switch-provider.md)

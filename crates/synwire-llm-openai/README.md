# synwire-llm-openai

OpenAI provider for Synwire. Implements `BaseChatModel` and `Embeddings` for the OpenAI API, including Azure OpenAI and compatible endpoints.

## What this crate provides

- **`ChatOpenAI`** — chat completions (GPT-4o, o3, o1, and any OpenAI-compatible model)
- **`OpenAIEmbeddings`** — text embeddings (text-embedding-3-small, text-embedding-3-large)
- **`BaseChatOpenAI`** — shared base for building OpenAI-compatible providers (Groq, Together, Perplexity)
- Streaming via SSE, tool binding, function calling, credential management via `secrecy`

## Quick start

```toml
[dependencies]
synwire-llm-openai = "0.1"
tokio = { version = "1", features = ["full"] }
```

```rust,no_run
use synwire_llm_openai::ChatOpenAI;
use synwire_core::language_models::chat::BaseChatModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    let result = model.invoke("Explain trait objects in Rust in one sentence.").await?;
    println!("{}", result.content);
    Ok(())
}
```

Streaming:

```rust,no_run
use synwire_llm_openai::ChatOpenAI;
use synwire_core::language_models::chat::BaseChatModel;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOpenAI::builder().model("gpt-4o").api_key_env("OPENAI_API_KEY").build()?;
    let mut stream = model.stream("Tell me a short story.").await?;
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?.content);
    }
    Ok(())
}
```

Azure OpenAI:

```rust,no_run
use synwire_llm_openai::ChatOpenAI;

let model = ChatOpenAI::builder()
    .model("gpt-4o")
    .api_base("https://my-resource.openai.azure.com/")
    .api_key_env("AZURE_OPENAI_KEY")
    .build()?;
```

Embeddings for RAG:

```rust,no_run
use synwire_llm_openai::OpenAIEmbeddings;
use synwire_core::embeddings::Embeddings;

let embeddings = OpenAIEmbeddings::builder()
    .model("text-embedding-3-small")
    .api_key_env("OPENAI_API_KEY")
    .build()?;

let vectors = embeddings.embed_documents(vec!["Rust is fast".to_string()]).await?;
```

## Documentation

- [LLM Providers Explanation](https://randomvariable.github.io/synwire/explanation/synwire-llm-providers.html)
- [How-To: Switch Provider](https://randomvariable.github.io/synwire/how-to/switch-provider.html)
- [Full API docs](https://docs.rs/synwire-llm-openai)
- [Synwire documentation](https://randomvariable.github.io/synwire/)

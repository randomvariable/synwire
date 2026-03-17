# LLM Providers: Choosing and Swapping

Synwire has two built-in LLM providers: `synwire-llm-openai` (cloud) and `synwire-llm-ollama` (local). Both implement `BaseChatModel` and `Embeddings`, so you can swap them by changing one line.

> **Background**: [Introduction to Agents](https://www.promptingguide.ai/agents/introduction) — the Prompt Engineering Guide covers how LLMs are used as the reasoning core of AI agents. Synwire's provider crates give you that reasoning core.

## `synwire-llm-openai`

Use when:
- You need GPT-4o, o3, o1, or another OpenAI model
- You are using Azure OpenAI (`api_base` override)
- You need OpenAI-compatible APIs (Groq, Together, Perplexity)

```rust,no_run
use synwire_llm_openai::ChatOpenAI;
use synwire_core::language_models::chat::BaseChatModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key_env("OPENAI_API_KEY")  // reads OPENAI_API_KEY at runtime
        .max_tokens(1024u16)
        .temperature(0.7)
        .build()?;

    let result = model.invoke("Explain Rust lifetimes in two sentences.").await?;
    println!("{}", result.content);
    Ok(())
}
```

**Builder methods**: `model`, `api_key`, `api_key_env`, `api_base`, `temperature`, `max_tokens`, `top_p`, `stop`, `timeout`, `max_retries`, `credential_provider`

## `synwire-llm-ollama`

Use when:
- All inference must stay **on your machine** — no data leaves the network boundary
- You are working **air-gapped** or in a privacy-sensitive environment
- You want zero API costs during development or testing

```rust,no_run
use synwire_llm_ollama::ChatOllama;
use synwire_core::language_models::chat::BaseChatModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Prerequisites: `ollama pull llama3.2`
    let model = ChatOllama::builder()
        .model("llama3.2")
        .build()?;

    let result = model.invoke("What is the borrow checker?").await?;
    println!("{}", result.content);
    Ok(())
}
```

**Builder methods**: `model`, `base_url` (default: `http://localhost:11434`), `temperature`, `top_k`, `top_p`, `num_predict`, `timeout`

## Swapping providers

Both providers implement `BaseChatModel`. Store the model as a trait object to swap by changing one line:

> 📖 **Rust note:** [`Box<dyn Trait>`](https://doc.rust-lang.org/book/ch15-01-box.html) heap-allocates a value and erases its concrete type, keeping only the trait interface. This is how Synwire stores different model implementations interchangeably.

```rust,no_run
use synwire_core::language_models::chat::BaseChatModel;
use synwire_llm_openai::ChatOpenAI;
use synwire_llm_ollama::ChatOllama;

fn build_model(use_local: bool) -> Box<dyn BaseChatModel> {
    if use_local {
        Box::new(ChatOllama::builder().model("llama3.2").build().unwrap())
    } else {
        Box::new(ChatOpenAI::builder().model("gpt-4o").api_key_env("OPENAI_API_KEY").build().unwrap())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = build_model(std::env::var("LOCAL").is_ok());
    let result = model.invoke("Hello").await?;
    println!("{}", result.content);
    Ok(())
}
```

## Embeddings

Both providers also implement `Embeddings`:

```rust,no_run
use synwire_core::embeddings::Embeddings;
use synwire_llm_openai::OpenAIEmbeddings;
use synwire_llm_ollama::OllamaEmbeddings;

// OpenAI embeddings
let openai_emb = OpenAIEmbeddings::builder()
    .model("text-embedding-3-small")
    .api_key_env("OPENAI_API_KEY")
    .build()?;

// Ollama embeddings (local, no API key)
let ollama_emb = OllamaEmbeddings::builder()
    .model("nomic-embed-text")
    .build()?;

// Both implement the same trait
let vectors = openai_emb.embed_query("Rust ownership").await?;
```

## Credential management

Never store API keys in plain `String` fields. Use `api_key_env` to read from the environment at runtime, or `credential_provider` for vault / secrets manager integration:

```rust,no_run
use synwire_llm_openai::ChatOpenAI;

let model = ChatOpenAI::builder()
    .model("gpt-4o")
    .credential_provider(|| {
        // Read from HashiCorp Vault, AWS Secrets Manager, etc.
        std::env::var("OPENAI_API_KEY").map_err(Into::into)
    })
    .build()?;
```

Keys are wrapped in `secrecy::Secret<String>` internally — they are never printed in logs or debug output.

## Implementing your own provider

Implement `BaseChatModel` in terms of `synwire-core` types only:

```rust,no_run
use synwire_core::language_models::chat::{BaseChatModel, ChatResult, ChatChunk};
use synwire_core::{BoxFuture, BoxStream, SynwireError};

struct MyProvider { api_url: String }

impl BaseChatModel for MyProvider {
    fn model_type(&self) -> &str { "my-provider" }

    fn invoke<'a>(&'a self, input: &'a str) -> BoxFuture<'a, Result<ChatResult, SynwireError>> {
        Box::pin(async move {
            // call self.api_url, parse response
            Ok(ChatResult { content: "response".to_string(), ..Default::default() })
        })
    }

    fn stream<'a>(&'a self, _input: &'a str) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, SynwireError>>, SynwireError>> {
        todo!()
    }
}
```

In tests, use `FakeChatModel` from `synwire-test-utils` instead of a real provider — it is deterministic and requires no network.

## See also

- [Local Inference with Ollama](../getting-started/ollama.md)
- [First Chat](../getting-started/first-chat.md)
- [How-To: Switch Provider](../how-to/switch-provider.md)
- [How-To: Credentials](../how-to/credentials.md)

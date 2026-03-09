# Quickstart: LangChain Rust

## Prerequisites

- Rust toolchain (stable, edition 2024)
- An OpenAI API key (for the provider example)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
langchain-core = "0.1"
langchain-openai = "0.1"    # Optional: for OpenAI provider
tokio = { version = "1", features = ["full"] }
```

## Example 1: Invoke a Chat Model

```rust
use langchain_core::prelude::*;
use langchain_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> Result<(), LangChainError> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::human("What is LangChain?"),
    ];

    let result = model.invoke(&messages, None).await?;
    println!("{}", result.message.content());
    Ok(())
}
```

## Example 2: Prompt Template + Chain

```rust
use langchain_core::prelude::*;
use langchain_openai::ChatOpenAI;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), LangChainError> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let template = ChatPromptTemplate::from_messages(vec![
        MessageTemplate::System("You are an expert on {topic}.".into()),
        MessageTemplate::Human("{question}".into()),
    ])?;

    let mut variables = HashMap::new();
    variables.insert("topic".to_string(), "Rust programming".into());
    variables.insert("question".to_string(), "What are traits?".into());

    let messages = template.format_messages(&variables)?;
    let result = model.invoke(&messages, None).await?;
    println!("{}", result.message.content());
    Ok(())
}
```

## Example 3: Streaming

```rust
use langchain_core::prelude::*;
use langchain_openai::ChatOpenAI;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), LangChainError> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let messages = vec![
        Message::human("Write a haiku about Rust."),
    ];

    let mut stream = model.stream(&messages, None).await?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        print!("{}", chunk.content());
    }
    println!();
    Ok(())
}
```

## Example 4: RAG (Retrieval-Augmented Generation)

```rust
use langchain_core::prelude::*;
use langchain_core::vectorstores::InMemoryVectorStore;
use langchain_openai::{ChatOpenAI, OpenAIEmbeddings};

#[tokio::main]
async fn main() -> Result<(), LangChainError> {
    let embeddings = OpenAIEmbeddings::builder()
        .model("text-embedding-3-small")
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let store = InMemoryVectorStore::new(Box::new(embeddings));

    let docs = vec![
        Document::new("Rust was created by Graydon Hoare at Mozilla."),
        Document::new("Rust uses ownership and borrowing for memory safety."),
        Document::new("Cargo is Rust's package manager and build tool."),
    ];
    store.add_documents(&docs, None).await?;

    let results = store.similarity_search("Who created Rust?", 2).await?;
    for doc in &results {
        println!("- {}", doc.page_content);
    }

    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let context: String = results.iter()
        .map(|d| d.page_content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let messages = vec![
        Message::system(&format!("Answer based on this context:\n{context}")),
        Message::human("Who created Rust?"),
    ];

    let result = model.invoke(&messages, None).await?;
    println!("\nAnswer: {}", result.message.content());
    Ok(())
}
```

## Verification

```bash
# Build the project
cargo build

# Run tests (no network required)
cargo test

# Run with OpenAI (requires OPENAI_API_KEY)
OPENAI_API_KEY=sk-... cargo run --example simple_chat
```

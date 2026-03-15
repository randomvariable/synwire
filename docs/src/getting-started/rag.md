# RAG (Retrieval-Augmented Generation)

This tutorial builds a RAG pipeline using vector stores, embeddings, and a retriever.

## Overview

A RAG pipeline:

1. Splits documents into chunks
2. Embeds chunks into a vector store
3. Retrieves relevant chunks for a query
4. Passes retrieved context to the model

## Dependencies

```toml
[dependencies]
synwire-core = "0.1"
synwire = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Full example with fake models

```rust,ignore
use synwire::text_splitters::RecursiveCharacterTextSplitter;
use synwire_core::documents::Document;
use synwire_core::embeddings::FakeEmbeddings;
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_core::messages::Message;
use synwire_core::vectorstores::{InMemoryVectorStore, VectorStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Split documents
    let splitter = RecursiveCharacterTextSplitter::new(100, 20);
    let text = "Rust is a systems programming language. \
                It emphasises safety and performance. \
                The ownership system prevents data races.";
    let chunks = splitter.split_text(text);

    // 2. Create documents from chunks
    let docs: Vec<Document> = chunks
        .into_iter()
        .map(|chunk| Document::new(chunk))
        .collect();

    // 3. Add to vector store
    let embeddings = FakeEmbeddings::new(32);
    let store = InMemoryVectorStore::new();
    let _ids = store.add_documents(&docs, &embeddings).await?;

    // 4. Retrieve relevant documents
    let results = store
        .similarity_search("What is Rust?", 2, &embeddings)
        .await?;

    // 5. Build context and query the model
    let context: String = results
        .iter()
        .map(|doc| doc.page_content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let model = FakeChatModel::new(vec![
        "Rust is a systems programming language focused on safety.".into(),
    ]);

    let prompt = format!(
        "Answer based on this context:\n{context}\n\nQuestion: What is Rust?"
    );
    let result = model.invoke(&[Message::human(&prompt)], None).await?;
    println!("{}", result.message.content().as_text());

    Ok(())
}
```

## Cached embeddings

Use `CacheBackedEmbeddings` to avoid recomputing embeddings:

```rust,ignore
use std::sync::Arc;
use synwire::cache::CacheBackedEmbeddings;
use synwire_core::embeddings::FakeEmbeddings;

let embeddings = Arc::new(FakeEmbeddings::new(32));
let cached = CacheBackedEmbeddings::new(embeddings, 1000); // cache up to 1000 entries
```

## Similarity search with scores

```rust,ignore
let scored_results = store
    .similarity_search_with_score("query", 5, &embeddings)
    .await?;

for (doc, score) in &scored_results {
    println!("Score: {score:.4} -- {}", doc.page_content);
}
```

## VectorStoreRetriever

For integration with the `Retriever` trait:

```rust,ignore
use synwire_core::retrievers::{VectorStoreRetriever, SearchType};

let retriever = VectorStoreRetriever::new(store, embeddings, SearchType::Similarity, 4);
```

## Next steps

- [Tools and Agents](./tools-agent.md) -- add tool-using agents
- [Graph Agents](./graph-agent.md) -- build stateful agent graphs

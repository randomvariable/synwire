//! Integration tests for `ChatOllama`.
//!
//! These tests require a running Ollama server at `localhost:11434` with the
//! `llama3.2` model pulled. They are marked `#[ignore]` so they do not run
//! in normal CI.
//!
//! Run with:
//! ```sh
//! cargo test -p synwire-llm-ollama --test integration_chat -- --ignored
//! ```

#![allow(clippy::expect_used, clippy::panic, clippy::print_stdout)]

use futures_util::StreamExt as _;
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::{Message, MessageContent};
use synwire_llm_ollama::ChatOllama;

#[tokio::test]
#[ignore = "requires running Ollama server with llama3.2 model"]
async fn test_chat_ollama_invoke() {
    let model = ChatOllama::builder()
        .model("llama3.2")
        .build()
        .expect("failed to build ChatOllama");

    let messages = vec![Message::human("Say hello in exactly three words.")];

    let result = model.invoke(&messages, None).await.expect("invoke failed");

    match &result.message {
        Message::AI {
            content: MessageContent::Text(text),
            ..
        } => {
            assert!(
                !text.is_empty(),
                "expected non-empty AI response, got empty string"
            );
            println!("Response: {text}");
        }
        other => {
            panic!("expected AI message with text content, got: {other:?}");
        }
    }
}

#[tokio::test]
#[ignore = "requires running Ollama server with llama3.2 model"]
async fn test_chat_ollama_stream() {
    let model = ChatOllama::builder()
        .model("llama3.2")
        .build()
        .expect("failed to build ChatOllama");

    let messages = vec![Message::human("Say hello in exactly three words.")];
    let mut stream = model
        .stream(&messages, None)
        .await
        .expect("stream creation failed");

    let mut full_content = String::new();
    let mut chunk_count: usize = 0;

    while let Some(result) = stream.next().await {
        let chunk = result.expect("stream chunk failed");
        if let Some(content) = &chunk.delta_content {
            full_content.push_str(content);
        }
        chunk_count += 1;
    }

    assert!(
        !full_content.is_empty(),
        "expected non-empty streamed content"
    );
    assert!(
        chunk_count > 0,
        "expected at least one chunk from the stream"
    );
    println!("Streamed ({chunk_count} chunks): {full_content}");
}

#[tokio::test]
#[ignore = "requires running Ollama server with nomic-embed-text model"]
async fn test_ollama_embeddings() {
    use synwire_core::embeddings::Embeddings;
    use synwire_llm_ollama::OllamaEmbeddings;

    let embeddings = OllamaEmbeddings::builder()
        .model("nomic-embed-text")
        .build()
        .expect("failed to build OllamaEmbeddings");

    let texts = vec!["Hello world".to_string(), "Goodbye world".to_string()];
    let result = embeddings
        .embed_documents(&texts)
        .await
        .expect("embed_documents failed");

    assert_eq!(result.len(), 2);
    assert!(!result[0].is_empty(), "expected non-empty embedding vector");
    println!(
        "Embedding dimensions: {}, first 3 values: {:?}",
        result[0].len(),
        &result[0][..3.min(result[0].len())]
    );
}

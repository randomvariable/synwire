//! Example: Few-shot RAG pipeline using fake models.
//!
//! Demonstrates: text splitting -> embedding -> few-shot retrieval -> model invocation.
//! Uses `FakeChatModel` and `FakeEmbeddings` to avoid API keys.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::collections::HashMap;
use std::sync::Arc;

use synwire::cache::CacheBackedEmbeddings;
use synwire::prompts::{ExampleSelector, FewShotPromptTemplate, SemanticSimilarityExampleSelector};
use synwire::text_splitters::RecursiveCharacterTextSplitter;
use synwire_core::embeddings::{Embeddings, FakeEmbeddings};
use synwire_core::language_models::FakeChatModel;
use synwire_core::language_models::traits::BaseChatModel;
use synwire_core::messages::Message;
use synwire_core::prompts::PromptTemplate;

#[tokio::main]
async fn main() {
    // 1. Split a document into chunks
    let document = "\
        Rust is a systems programming language focused on safety and performance.\n\n\
        It was originally designed by Graydon Hoare at Mozilla Research.\n\n\
        Rust prevents null pointer dereferences and data races at compile time.\n\n\
        The language uses an ownership system with borrowing and lifetimes.";

    let splitter = RecursiveCharacterTextSplitter::new(80, 10);
    let chunks = splitter.split_text(document);
    println!("Split document into {} chunks:", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        println!("  Chunk {i}: {chunk}");
    }

    // 2. Embed chunks using cached embeddings
    let embeddings = Arc::new(FakeEmbeddings::new(32));
    let cached_embeddings = CacheBackedEmbeddings::new(embeddings, 100);

    let vectors = cached_embeddings.embed_documents(&chunks).await.unwrap();
    println!(
        "\nEmbedded {} chunks (dim={})",
        vectors.len(),
        vectors[0].len()
    );

    // 3. Build few-shot examples from the chunks
    let selector = SemanticSimilarityExampleSelector::new();
    for chunk in &chunks {
        let mut example = HashMap::new();
        let _ = example.insert("context".into(), chunk.clone());
        let _ = example.insert("answer".into(), format!("Based on: {chunk}"));
        selector.add_example(example).await.unwrap();
    }
    let selector = Arc::new(selector);

    // 4. Create a few-shot prompt template
    let example_template = PromptTemplate::new(
        "Context: {context}\nAnswer: {answer}",
        vec!["context".into(), "answer".into()],
    );

    let few_shot = FewShotPromptTemplate::with_selector(
        selector,
        example_template,
        "Answer questions about Rust using these examples:\n\n{examples}\n\nQuestion: {question}\nAnswer:",
        vec!["question".into()],
    );

    let mut vars = HashMap::new();
    let _ = vars.insert("question".into(), "What is Rust?".into());
    let prompt = few_shot.format(&vars).await.unwrap();
    println!("\nFormatted prompt:\n{prompt}");

    // 5. Invoke a fake model
    let model = FakeChatModel::new(vec![
        "Rust is a systems programming language that emphasizes safety and performance.".into(),
    ]);

    let messages = vec![Message::human(prompt)];
    let result = model.invoke(&messages, None).await.unwrap();
    println!("\nModel response: {}", result.message.content().as_text());
}

//! E2E test: Retrieval-Augmented Generation pipeline.
//!
//! Requires a running Ollama instance. Run with `cargo nextest run --run-ignored all`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[ignore = "requires running Ollama instance"]
#[tokio::test]
async fn e2e_rag_pipeline() {
    // Stub: when RAG pipeline is wired, this test will:
    // 1. Create an InMemoryVectorStore with FakeEmbeddings
    // 2. Add several documents
    // 3. Create a retrieval chain
    // 4. Query with a question
    // 5. Verify the response references the relevant documents
}

#[ignore = "requires running Ollama instance"]
#[tokio::test]
async fn e2e_rag_with_metadata_filter() {
    // Stub: test RAG with metadata filtering.
}

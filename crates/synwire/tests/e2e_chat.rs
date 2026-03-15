//! E2E test: Chat completion via Ollama.
//!
//! Requires a running Ollama instance. Run with `cargo nextest run --run-ignored all`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[ignore = "requires running Ollama instance"]
#[tokio::test]
async fn e2e_chat_completion() {
    // Stub: when Ollama integration is wired, this test will:
    // 1. Connect to Ollama at OLLAMA_HOST (default localhost:11434)
    // 2. Send a simple chat message
    // 3. Verify a non-empty response is returned
}

#[ignore = "requires running Ollama instance"]
#[tokio::test]
async fn e2e_chat_streaming() {
    // Stub: test streaming chat completion.
    // 1. Connect to Ollama
    // 2. Send a message with streaming enabled
    // 3. Collect stream chunks
    // 4. Verify concatenated chunks form a valid response
}

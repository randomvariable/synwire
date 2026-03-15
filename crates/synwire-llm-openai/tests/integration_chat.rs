//! Integration tests for `ChatOpenAI`.
//!
//! These tests require a valid `OPENAI_API_KEY` environment variable and make
//! real API calls. They are marked `#[ignore]` so they do not run in normal CI.
//!
//! Run with:
//! ```sh
//! cargo test -p synwire-llm-openai --test integration_chat -- --ignored
//! ```

#![allow(clippy::expect_used, clippy::panic, clippy::print_stdout)]

use futures_util::StreamExt as _;
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::{Message, MessageContent};
use synwire_llm_openai::ChatOpenAI;

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn test_chat_openai_invoke() {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key_env("OPENAI_API_KEY")
        .build()
        .expect("failed to build ChatOpenAI — is OPENAI_API_KEY set?");

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
        }
        other => {
            panic!("expected AI message with text content, got: {other:?}");
        }
    }
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn test_chat_openai_invoke_with_system_message() {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key_env("OPENAI_API_KEY")
        .temperature(0.0)
        .build()
        .expect("failed to build ChatOpenAI");

    let messages = vec![
        Message::system("You are a helpful assistant. Always respond with exactly one word."),
        Message::human("What color is the sky on a clear day?"),
    ];

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
        }
        other => {
            panic!("expected AI message with text content, got: {other:?}");
        }
    }
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn test_chat_openai_stream() {
    let model = ChatOpenAI::builder()
        .model("gpt-4o-mini")
        .api_key_env("OPENAI_API_KEY")
        .build()
        .expect("failed to build ChatOpenAI — is OPENAI_API_KEY set?");

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
}

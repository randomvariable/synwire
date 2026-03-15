//! Simple chat example using `ChatOpenAI`.
//!
//! Demonstrates how to build a `ChatOpenAI` model, send a human message,
//! and print the AI response.
//!
//! # Prerequisites
//!
//! Set the `OPENAI_API_KEY` environment variable before running:
//!
//! ```sh
//! export OPENAI_API_KEY="sk-..."
//! cargo run --example simple_chat -p synwire-llm-openai
//! ```

#![allow(clippy::print_stdout)]

use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::{Message, MessageContent};
use synwire_llm_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    let messages = vec![Message::human("Hello, how are you?")];

    let result = model.invoke(&messages, None).await?;

    match &result.message {
        Message::AI {
            content: MessageContent::Text(text),
            ..
        } => println!("{text}"),
        _ => println!("Unexpected response format"),
    }

    Ok(())
}

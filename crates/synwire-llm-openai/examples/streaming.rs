//! Example: Stream `ChatOpenAI` responses token by token.
//!
//! Requires `OPENAI_API_KEY` environment variable.
//!
//! Run with:
//! ```sh
//! OPENAI_API_KEY=sk-... cargo run -p synwire-llm-openai --example streaming
//! ```

#![allow(clippy::print_stdout)]

use futures_util::StreamExt as _;
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;
use synwire_llm_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o-mini")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    let messages = vec![Message::human("Write a haiku about Rust programming")];
    let mut stream = model.stream(&messages, None).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = &chunk.delta_content {
            print!("{content}");
        }
    }
    println!();

    Ok(())
}

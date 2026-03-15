//! Example: Prompt template -> `ChatOpenAI` -> `StrOutputParser`
//!
//! Demonstrates composing a prompt template with a model and parser.
//! Requires `OPENAI_API_KEY` environment variable.
//!
//! # Usage
//!
//! ```sh
//! export OPENAI_API_KEY="sk-..."
//! cargo run --example prompt_chain -p synwire-llm-openai
//! ```

#![allow(clippy::print_stdout)]

use std::collections::HashMap;

use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;
use synwire_core::output_parsers::{OutputParser, StrOutputParser};
use synwire_core::prompts::PromptTemplate;
use synwire_llm_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a prompt template
    let template = PromptTemplate::new("Tell me a joke about {topic}", vec!["topic".into()]);

    // 2. Format the prompt
    let mut vars = HashMap::new();
    let _prev = vars.insert("topic".into(), "Rust programming".into());
    let prompt = template.format(&vars)?;

    // 3. Create the model
    let model = ChatOpenAI::builder()
        .model("gpt-4o-mini")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    // 4. Invoke the model
    let messages = vec![Message::human(&prompt)];
    let result = model.invoke(&messages, None).await?;

    // 5. Parse the output
    let parser = StrOutputParser;
    let output = parser.parse(&result.message.content().as_text())?;

    println!("Joke: {output}");

    Ok(())
}

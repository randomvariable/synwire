# First Chat

This tutorial walks through installing Synwire and making your first chat model invocation.

## Add dependencies

Add the following to your `Cargo.toml`:

```toml
[dependencies]
synwire-core = "0.1"
tokio = { version = "1", features = ["full"] }
```

For testing without API keys, `synwire-core` includes `FakeChatModel`. To use a real provider, add `synwire-llm-openai` or `synwire-llm-ollama`.

## Using FakeChatModel (no API key)

```rust,ignore
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_core::messages::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // FakeChatModel returns pre-configured responses in order
    let model = FakeChatModel::new(vec![
        "I'm doing well, thanks for asking!".into(),
    ]);

    let messages = vec![Message::human("How are you?")];
    let result = model.invoke(&messages, None).await?;

    println!("{}", result.message.content().as_text());
    // Output: I'm doing well, thanks for asking!

    Ok(())
}
```

## Using ChatOpenAI

Add `synwire-llm-openai` to your dependencies:

```toml
[dependencies]
synwire-llm-openai = "0.1"
```

```rust,ignore
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;
use synwire_llm_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o-mini")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    let messages = vec![Message::human("What is Rust?")];
    let result = model.invoke(&messages, None).await?;

    println!("{}", result.message.content().as_text());

    Ok(())
}
```

## Multi-turn conversations

All chat models accept a slice of `Message` values. Build a conversation by including system, human, and AI messages:

```rust,ignore
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_core::messages::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = FakeChatModel::new(vec![
        "Paris is the capital of France.".into(),
    ]);

    let messages = vec![
        Message::system("You are a helpful geography assistant."),
        Message::human("What is the capital of France?"),
    ];

    let result = model.invoke(&messages, None).await?;
    println!("{}", result.message.content().as_text());

    Ok(())
}
```

## Batch invocation

Call `batch` to invoke on multiple inputs:

```rust,ignore
let inputs = vec![
    vec![Message::human("Question 1")],
    vec![Message::human("Question 2")],
];
let results = model.batch(&inputs, None).await?;
for r in &results {
    println!("{}", r.message.content().as_text());
}
```

## Key types

| Type | Purpose |
|------|---------|
| `BaseChatModel` | Trait for all chat models |
| `Message` | Conversation message (human, AI, system, tool) |
| `ChatResult` | Model response including the AI message |
| `FakeChatModel` | Deterministic model for testing |
| `RunnableConfig` | Optional configuration (callbacks, tags, metadata) |

## Next steps

- [Prompt Chains](./prompt-chains.md) -- compose templates with models
- [Streaming](./streaming.md) -- stream responses token by token

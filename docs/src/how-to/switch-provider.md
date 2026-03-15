# Switch Provider

All chat models implement the `BaseChatModel` trait, so switching providers requires only changing the constructor.

## Using trait objects

```rust,ignore
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;

async fn ask(
    model: &dyn BaseChatModel,
    question: &str,
) -> Result<String, synwire_core::error::SynwireError> {
    let messages = vec![Message::human(question)];
    let result = model.invoke(&messages, None).await?;
    Ok(result.message.content().as_text())
}
```

## Selecting at runtime

```rust,ignore
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_llm_openai::ChatOpenAI;
use synwire_llm_ollama::ChatOllama;

fn create_model(provider: &str) -> Result<Box<dyn BaseChatModel>, Box<dyn std::error::Error>> {
    match provider {
        "openai" => Ok(Box::new(
            ChatOpenAI::builder()
                .model("gpt-4o-mini")
                .api_key_env("OPENAI_API_KEY")
                .build()?
        )),
        "ollama" => Ok(Box::new(
            ChatOllama::builder()
                .model("llama3.2")
                .build()?
        )),
        "fake" => Ok(Box::new(
            FakeChatModel::new(vec!["Fake response".into()])
        )),
        _ => Err("Unknown provider".into()),
    }
}
```

## Feature flags for optional providers

Use Cargo feature flags to conditionally compile providers:

```toml
[dependencies]
synwire = "0.1"

[features]
default = []
openai = ["synwire/openai"]
ollama = ["synwire/ollama"]
```

## Testing with FakeChatModel

Replace any real provider with `FakeChatModel` in tests:

```rust,ignore
#[cfg(test)]
mod tests {
    use synwire_core::language_models::{FakeChatModel, BaseChatModel};

    #[tokio::test]
    async fn test_my_agent() {
        let model: Box<dyn BaseChatModel> = Box::new(
            FakeChatModel::new(vec!["Test response".into()])
        );
        // Use model in your agent...
    }
}
```

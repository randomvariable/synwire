# Custom Provider

Implement `BaseChatModel` to add support for a new LLM provider.

## Implement the trait

```rust,ignore
use synwire_core::error::SynwireError;
use synwire_core::language_models::{BaseChatModel, ChatResult, ChatChunk};
use synwire_core::messages::Message;
use synwire_core::runnables::RunnableConfig;
use synwire_core::tools::ToolSchema;
use synwire_core::{BoxFuture, BoxStream};

pub struct MyProvider {
    model_name: String,
    api_key: String,
}

impl BaseChatModel for MyProvider {
    fn invoke<'a>(
        &'a self,
        messages: &'a [Message],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<ChatResult, SynwireError>> {
        Box::pin(async move {
            // Make API call to your provider...
            let response_text = "Response from my provider";
            Ok(ChatResult {
                message: Message::ai(response_text),
                generation_info: None,
                cost: None,
            })
        })
    }

    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, SynwireError>>, SynwireError>> {
        Box::pin(async move {
            // Return a stream of ChatChunk values...
            let chunks = vec![Ok(ChatChunk {
                delta_content: Some("Response".into()),
                delta_tool_calls: Vec::new(),
                finish_reason: Some("stop".into()),
                usage: None,
            })];
            Ok(Box::pin(futures_util::stream::iter(chunks))
                as BoxStream<'_, Result<ChatChunk, SynwireError>>)
        })
    }

    fn model_type(&self) -> &str {
        "my-provider"
    }
}
```

## Builder pattern

Use the builder pattern for ergonomic construction:

```rust,ignore
pub struct MyProviderBuilder {
    model: Option<String>,
    api_key: Option<String>,
}

impl MyProviderBuilder {
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn api_key_env(mut self, env_var: &str) -> Self {
        self.api_key = std::env::var(env_var).ok();
        self
    }

    pub fn build(self) -> Result<MyProvider, SynwireError> {
        Ok(MyProvider {
            model_name: self.model.unwrap_or_else(|| "default".into()),
            api_key: self.api_key.ok_or(SynwireError::Credential {
                message: "API key required".into(),
            })?,
        })
    }
}
```

## Embeddings provider

Implement `Embeddings` for embedding support:

```rust,ignore
use synwire_core::embeddings::Embeddings;
use synwire_core::error::SynwireError;
use synwire_core::BoxFuture;

impl Embeddings for MyProvider {
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, SynwireError>> {
        Box::pin(async move {
            // Call embedding API...
            Ok(vec![vec![0.0; 768]; texts.len()])
        })
    }

    fn embed_query<'a>(
        &'a self,
        text: &'a str,
    ) -> BoxFuture<'a, Result<Vec<f32>, SynwireError>> {
        Box::pin(async move {
            Ok(vec![0.0; 768])
        })
    }
}
```

## Testing your provider

Use `FakeChatModel` as a reference for expected behaviour, and write tests against the `BaseChatModel` trait:

```rust,ignore
#[tokio::test]
async fn test_invoke_returns_ai_message() {
    let model = MyProvider::builder()
        .model("test")
        .build()
        .unwrap();
    let result = model.invoke(&[Message::human("Hi")], None).await.unwrap();
    assert_eq!(result.message.message_type(), "ai");
}
```

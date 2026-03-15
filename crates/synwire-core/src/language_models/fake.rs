//! Fake chat model for testing.

use crate::BoxFuture;
use crate::BoxStream;
use crate::error::{ModelError, SynwireError};
use crate::language_models::traits::BaseChatModel;
use crate::language_models::types::{ChatChunk, ChatResult};
use crate::messages::Message;
use crate::runnables::RunnableConfig;
use crate::tools::ToolSchema;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fake chat model for testing without API calls.
///
/// Returns pre-configured responses in order. Tracks call count
/// and can inject errors at specified positions.
///
/// # Examples
///
/// ```
/// use synwire_core::language_models::fake::FakeChatModel;
/// use synwire_core::language_models::traits::BaseChatModel;
/// use synwire_core::messages::Message;
///
/// # tokio_test::block_on(async {
/// let model = FakeChatModel::new(vec!["Hello!".into()]);
/// let messages = vec![Message::human("Hi")];
/// let result = model.invoke(&messages, None).await.unwrap();
/// assert_eq!(result.message.content().as_text(), "Hello!");
/// # });
/// ```
pub struct FakeChatModel {
    responses: Vec<String>,
    call_count: AtomicUsize,
    error_at: Option<usize>,
    calls: Mutex<Vec<Vec<Message>>>,
    /// When `Some(n)`, the stream method splits each response into chunks
    /// of `n` characters instead of yielding the entire text as one chunk.
    chunk_size: Option<usize>,
    /// When `Some(n)`, the stream method injects an error after yielding
    /// `n` chunks, causing all subsequent chunks to be errors.
    stream_error_after: Option<usize>,
}

impl FakeChatModel {
    /// Creates a new fake chat model with the given responses.
    ///
    /// Responses are returned in order, cycling back to the start
    /// when all responses have been used.
    pub const fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: AtomicUsize::new(0),
            error_at: None,
            calls: Mutex::new(Vec::new()),
            chunk_size: None,
            stream_error_after: None,
        }
    }

    /// Sets the call index at which to return an error.
    ///
    /// When the zero-based call count matches `index`, the model
    /// returns a [`ModelError::Other`] instead of a response.
    #[must_use]
    pub const fn with_error_at(mut self, index: usize) -> Self {
        self.error_at = Some(index);
        self
    }

    /// Sets the chunk size for streaming responses.
    ///
    /// When set, the [`stream`](BaseChatModel::stream) method splits each
    /// response into chunks of at most `size` characters.
    #[must_use]
    pub const fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = Some(size);
        self
    }

    /// Configures an error to be injected after `n` chunks during streaming.
    ///
    /// The stream yields the first `n` chunks successfully, then returns
    /// an error for every subsequent chunk position.
    #[must_use]
    pub const fn with_stream_error_after(mut self, n: usize) -> Self {
        self.stream_error_after = Some(n);
        self
    }

    /// Returns the number of times invoke has been called.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Returns a clone of all recorded input message lists.
    pub fn calls(&self) -> Vec<Vec<Message>> {
        self.calls.lock().map_or_else(|_| Vec::new(), |g| g.clone())
    }
}

impl std::fmt::Debug for FakeChatModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeChatModel")
            .field("responses", &self.responses)
            .field("call_count", &self.call_count.load(Ordering::Relaxed))
            .field("error_at", &self.error_at)
            .field("calls", &self.calls)
            .field("chunk_size", &self.chunk_size)
            .field("stream_error_after", &self.stream_error_after)
            .finish()
    }
}

impl BaseChatModel for FakeChatModel {
    fn invoke<'a>(
        &'a self,
        messages: &'a [Message],
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<ChatResult, SynwireError>> {
        Box::pin(async move {
            let idx = self.call_count.fetch_add(1, Ordering::Relaxed);

            // Record call
            if let Ok(mut calls) = self.calls.lock() {
                calls.push(messages.to_vec());
            }

            // Check for error injection
            if self.error_at == Some(idx) {
                return Err(SynwireError::from(ModelError::Other {
                    message: format!("injected error at call {idx}"),
                }));
            }

            let response_text = self
                .responses
                .get(idx % self.responses.len())
                .cloned()
                .unwrap_or_default();

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
            let result = self.invoke(messages, config).await?;
            let full_text = result.message.content().as_text();

            let chunk_size = self.chunk_size.unwrap_or(full_text.len()).max(1);
            let error_after = self.stream_error_after;

            let chunks: Vec<String> = full_text
                .chars()
                .collect::<Vec<_>>()
                .chunks(chunk_size)
                .map(|c| c.iter().collect())
                .collect();

            let total = chunks.len();
            let stream =
                futures_util::stream::iter(chunks.into_iter().enumerate().map(move |(i, text)| {
                    if let Some(error_at) = error_after {
                        if i >= error_at {
                            return Err(SynwireError::from(ModelError::Other {
                                message: "stream error injected".into(),
                            }));
                        }
                    }
                    let finish_reason = if i + 1 == total {
                        Some("stop".into())
                    } else {
                        None
                    };
                    Ok(ChatChunk {
                        delta_content: Some(text),
                        delta_tool_calls: Vec::new(),
                        finish_reason,
                        usage: None,
                    })
                }));

            Ok(Box::pin(stream) as BoxStream<'_, Result<ChatChunk, SynwireError>>)
        })
    }

    fn model_type(&self) -> &'static str {
        "fake"
    }

    fn bind_tools(&self, _tools: &[ToolSchema]) -> Result<Box<dyn BaseChatModel>, SynwireError> {
        let mut model = Self::new(self.responses.clone());
        model.chunk_size = self.chunk_size;
        model.stream_error_after = self.stream_error_after;
        Ok(Box::new(model))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_chat_model_invoke_returns_chat_result() {
        let model = FakeChatModel::new(vec!["Hello!".into()]);
        let messages = vec![Message::human("Hi")];
        let result = model.invoke(&messages, None).await.unwrap();
        assert_eq!(result.message.content().as_text(), "Hello!");
        assert_eq!(result.message.message_type(), "ai");
    }

    #[tokio::test]
    async fn test_fake_chat_model_invoke_with_error() {
        let model = FakeChatModel::new(vec!["ok".into()]).with_error_at(0);
        let messages = vec![Message::human("Hi")];
        let result = model.invoke(&messages, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fake_chat_model_swap_compiles() {
        let model_a: Box<dyn BaseChatModel> = Box::new(FakeChatModel::new(vec!["A".into()]));
        let model_b: Box<dyn BaseChatModel> = Box::new(FakeChatModel::new(vec!["B".into()]));
        let messages = vec![Message::human("test")];

        let result_a = model_a.invoke(&messages, None).await.unwrap();
        let result_b = model_b.invoke(&messages, None).await.unwrap();
        assert_eq!(result_a.message.content().as_text(), "A");
        assert_eq!(result_b.message.content().as_text(), "B");
    }

    #[tokio::test]
    async fn test_fake_chat_model_batch() {
        let model = FakeChatModel::new(vec!["R1".into(), "R2".into()]);
        let inputs = vec![vec![Message::human("Q1")], vec![Message::human("Q2")]];
        let results = model.batch(&inputs, None).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].message.content().as_text(), "R1");
        assert_eq!(results[1].message.content().as_text(), "R2");
    }

    #[tokio::test]
    async fn test_invoke_empty_messages_returns_result() {
        let model = FakeChatModel::new(vec!["response".into()]);
        let result = model.invoke(&[], None).await.unwrap();
        assert_eq!(result.message.content().as_text(), "response");
    }

    #[tokio::test]
    async fn test_bind_tools_returns_model() {
        let model = FakeChatModel::new(vec!["ok".into()]);
        let tools = vec![crate::tools::ToolSchema {
            name: "search".into(),
            description: "Search".into(),
            parameters: serde_json::json!({}),
        }];
        let bound = model.bind_tools(&tools).unwrap();
        assert_eq!(bound.model_type(), "fake");
    }

    #[tokio::test]
    async fn test_call_tracking() {
        let model = FakeChatModel::new(vec!["A".into(), "B".into()]);
        let _r1 = model.invoke(&[Message::human("Q1")], None).await.unwrap();
        let _r2 = model.invoke(&[Message::human("Q2")], None).await.unwrap();
        assert_eq!(model.call_count(), 2);
        let calls = model.calls();
        assert_eq!(calls.len(), 2);
    }

    #[tokio::test]
    async fn test_fake_stream_yields_chunks_in_order() {
        use futures_util::StreamExt as _;

        let model = FakeChatModel::new(vec!["abcdefgh".into()]).with_chunk_size(3);
        let messages = vec![Message::human("Hi")];
        let mut stream = model.stream(&messages, None).await.unwrap();

        let mut chunks = Vec::new();
        while let Some(result) = stream.next().await {
            let chunk = result.unwrap();
            if let Some(text) = &chunk.delta_content {
                chunks.push(text.clone());
            }
        }

        assert_eq!(chunks, vec!["abc", "def", "gh"]);
    }

    #[tokio::test]
    async fn test_concatenated_stream_equals_invoke() {
        use futures_util::StreamExt as _;

        let response = "Hello, this is a test response!";
        let model = FakeChatModel::new(vec![response.into()]).with_chunk_size(5);
        let messages = vec![Message::human("Hi")];

        // Stream and concatenate
        let mut stream = model.stream(&messages, None).await.unwrap();
        let mut streamed = String::new();
        while let Some(result) = stream.next().await {
            let chunk = result.unwrap();
            if let Some(text) = &chunk.delta_content {
                streamed.push_str(text);
            }
        }

        // Invoke (call_count is now 1 from stream's internal invoke, so
        // the second call cycles to index 1 % 1 == 0, returning the same response)
        let invoke_result = model.invoke(&messages, None).await.unwrap();
        let invoked = invoke_result.message.content().as_text();

        assert_eq!(streamed, invoked);
    }

    #[tokio::test]
    async fn test_stream_mid_error() {
        use futures_util::StreamExt as _;

        let model = FakeChatModel::new(vec!["abcdefghij".into()])
            .with_chunk_size(2)
            .with_stream_error_after(2);

        let messages = vec![Message::human("Hi")];
        let mut stream = model.stream(&messages, None).await.unwrap();

        let mut ok_chunks = Vec::new();
        let mut saw_error = false;

        while let Some(result) = stream.next().await {
            if let Ok(chunk) = result {
                if let Some(text) = &chunk.delta_content {
                    ok_chunks.push(text.clone());
                }
            } else {
                saw_error = true;
                break;
            }
        }

        assert_eq!(ok_chunks, vec!["ab", "cd"]);
        assert!(saw_error, "expected an error after 2 chunks");
    }

    #[tokio::test]
    async fn test_stream_drop_no_leak() {
        use futures_util::StreamExt as _;

        let model = FakeChatModel::new(vec!["abcdefghij".into()]).with_chunk_size(2);
        let messages = vec![Message::human("Hi")];
        let mut stream = model.stream(&messages, None).await.unwrap();

        // Consume only the first chunk, then drop the stream
        let first = stream.next().await;
        assert!(first.is_some());
        drop(stream);
        // No panic or resource leak -- test passes by completing successfully
    }

    #[tokio::test]
    async fn test_runnable_core_default_stream() {
        use crate::runnables::core::RunnableCore;
        use futures_util::StreamExt as _;

        struct EchoRunnable;

        impl RunnableCore for EchoRunnable {
            fn invoke<'a>(
                &'a self,
                input: serde_json::Value,
                _config: Option<&'a crate::runnables::RunnableConfig>,
            ) -> crate::BoxFuture<'a, Result<serde_json::Value, crate::error::SynwireError>>
            {
                Box::pin(async move { Ok(input) })
            }
        }

        let runnable = EchoRunnable;
        let input = serde_json::json!({"greeting": "hello"});
        let mut stream = runnable.stream(input.clone(), None).await.unwrap();

        let first = stream.next().await;
        assert!(first.is_some());
        let value = first.unwrap().unwrap();
        assert_eq!(value, input);

        // Should have no more items
        let second = stream.next().await;
        assert!(second.is_none());
    }
}

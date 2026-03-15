//! Language model trait definitions.

use crate::BoxFuture;
use crate::BoxStream;
use crate::error::SynwireError;
use crate::language_models::types::{ChatChunk, ChatResult};
use crate::messages::Message;
use crate::runnables::RunnableConfig;
use crate::tools::ToolSchema;

/// Base trait for chat language models.
///
/// All chat models must implement this trait. Methods use manual
/// `BoxFuture` desugaring for dyn-compatibility.
///
/// # Cancel safety
///
/// The futures returned by [`invoke`](Self::invoke), [`batch`](Self::batch),
/// and [`stream`](Self::stream) are **not cancel-safe** in general.
/// Dropping a future mid-execution may leave the underlying HTTP connection
/// in an indeterminate state. If you need cancellation, use
/// [`tokio::time::timeout`] and create a fresh request on timeout.
/// The [`BoxStream`] returned by `stream` can be safely dropped at any
/// point; unread chunks are simply discarded.
pub trait BaseChatModel: Send + Sync {
    /// Invoke the model with a list of messages.
    fn invoke<'a>(
        &'a self,
        messages: &'a [Message],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<ChatResult, SynwireError>>;

    /// Invoke the model on multiple inputs concurrently.
    fn batch<'a>(
        &'a self,
        inputs: &'a [Vec<Message>],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<ChatResult>, SynwireError>> {
        Box::pin(async move {
            let mut results = Vec::with_capacity(inputs.len());
            for messages in inputs {
                results.push(self.invoke(messages, config).await?);
            }
            Ok(results)
        })
    }

    /// Stream model responses as incremental chunks.
    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, SynwireError>>, SynwireError>>;

    /// Returns the model type identifier.
    fn model_type(&self) -> &str;

    /// Returns a new model instance with tools bound.
    fn bind_tools(&self, _tools: &[ToolSchema]) -> Result<Box<dyn BaseChatModel>, SynwireError> {
        Err(SynwireError::Prompt {
            message: "bind_tools not supported by this model".into(),
        })
    }
}

/// Base trait for text completion language models.
///
/// For non-chat (completion-style) LLMs.
pub trait BaseLLM: Send + Sync {
    /// Invoke the model with a text prompt.
    fn invoke<'a>(
        &'a self,
        prompt: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<String, SynwireError>>;

    /// Invoke on multiple prompts.
    fn batch<'a>(
        &'a self,
        prompts: &'a [String],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<String>, SynwireError>> {
        Box::pin(async move {
            let mut results = Vec::with_capacity(prompts.len());
            for prompt in prompts {
                results.push(self.invoke(prompt, config).await?);
            }
            Ok(results)
        })
    }

    /// Stream responses as incremental text chunks.
    fn stream<'a>(
        &'a self,
        prompt: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<String, SynwireError>>, SynwireError>>;

    /// Returns the model type identifier.
    fn model_type(&self) -> &str;
}

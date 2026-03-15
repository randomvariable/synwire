//! `ChatOpenAI` implementation.

use crate::base::{BaseChatOpenAI, BaseChatOpenAIBuilder};
use crate::error::OpenAIError;
use eventsource_stream::Eventsource as _;
use futures_util::StreamExt as _;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use synwire_core::credentials::CredentialProvider;
use synwire_core::error::SynwireError;
use synwire_core::language_models::traits::BaseChatModel;
use synwire_core::language_models::{ChatChunk, ChatResult, ToolCallChunk};
use synwire_core::messages::{Message, MessageContent, ToolCall, UsageMetadata};
use synwire_core::runnables::RunnableConfig;
use synwire_core::tools::ToolSchema;
use synwire_core::{BoxFuture, BoxStream};

/// `OpenAI` chat model provider.
///
/// # Example
/// ```no_run
/// use synwire_llm_openai::ChatOpenAI;
///
/// let model = ChatOpenAI::builder()
///     .model("gpt-4o")
///     .api_key("sk-...")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct ChatOpenAI {
    pub(crate) base: BaseChatOpenAI,
    pub(crate) tools: Vec<ToolSchema>,
}

impl ChatOpenAI {
    /// Returns a builder for constructing a [`ChatOpenAI`] instance.
    pub fn builder() -> ChatOpenAIBuilder {
        ChatOpenAIBuilder {
            base: BaseChatOpenAIBuilder::new(),
            tools: Vec::new(),
        }
    }
}

/// Builder for [`ChatOpenAI`].
#[derive(Debug)]
pub struct ChatOpenAIBuilder {
    base: BaseChatOpenAIBuilder,
    tools: Vec<ToolSchema>,
}

impl ChatOpenAIBuilder {
    /// Sets the model name.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.base = self.base.model(model);
        self
    }

    /// Sets the API key.
    #[must_use]
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.base = self.base.api_key(key);
        self
    }

    /// Sets the API base URL.
    #[must_use]
    pub fn api_base(mut self, url: impl Into<String>) -> Self {
        self.base = self.base.api_base(url);
        self
    }

    /// Sets the temperature.
    #[must_use]
    pub fn temperature(mut self, temp: f32) -> Self {
        self.base = self.base.temperature(temp);
        self
    }

    /// Sets the max tokens.
    #[must_use]
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.base = self.base.max_tokens(max);
        self
    }

    /// Sets the API key environment variable name.
    #[must_use]
    pub fn api_key_env(mut self, env_var: impl Into<String>) -> Self {
        self.base = self.base.api_key_env(env_var);
        self
    }

    /// Sets the top-p sampling parameter.
    #[must_use]
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.base = self.base.top_p(top_p);
        self
    }

    /// Sets the stop sequences.
    #[must_use]
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.base = self.base.stop(stop);
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.base = self.base.timeout(timeout);
        self
    }

    /// Sets the maximum number of retries.
    #[must_use]
    pub fn max_retries(mut self, max: u32) -> Self {
        self.base = self.base.max_retries(max);
        self
    }

    /// Sets a credential provider for dynamic credential refresh on 401/403.
    #[must_use]
    pub fn credential_provider(mut self, provider: Arc<dyn CredentialProvider>) -> Self {
        self.base = self.base.credential_provider(provider);
        self
    }

    /// Builds the [`ChatOpenAI`] instance.
    pub fn build(self) -> Result<ChatOpenAI, SynwireError> {
        let base = self.base.build()?;
        Ok(ChatOpenAI {
            base,
            tools: self.tools,
        })
    }
}

impl BaseChatModel for ChatOpenAI {
    fn invoke<'a>(
        &'a self,
        messages: &'a [Message],
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<ChatResult, SynwireError>> {
        Box::pin(async move {
            let tools = if self.tools.is_empty() {
                None
            } else {
                Some(self.tools.as_slice())
            };
            let body = self.base.build_request_body(messages, false, tools);
            let url = self.base.completions_url();

            let response = self.base.send_with_auth_retry(&url, &body).await?;

            let response_json: Value = response
                .json()
                .await
                .map_err(|e| SynwireError::from(OpenAIError::ResponseParse(e.to_string())))?;

            parse_chat_response(&response_json)
        })
    }

    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, SynwireError>>, SynwireError>> {
        Box::pin(async move {
            let tools = if self.tools.is_empty() {
                None
            } else {
                Some(self.tools.as_slice())
            };
            let body = self.base.build_request_body(messages, true, tools);
            let url = self.base.completions_url();

            let response = self.base.send_with_auth_retry(&url, &body).await?;

            let byte_stream = response.bytes_stream();
            let event_stream = byte_stream.eventsource();

            let chunk_stream = futures_util::stream::unfold(event_stream, |mut es| async move {
                loop {
                    match es.next().await {
                        Some(Ok(event)) => {
                            if event.data == "[DONE]" {
                                return None;
                            }
                            match serde_json::from_str::<Value>(&event.data) {
                                Ok(json) => {
                                    if let Some(chunk) = parse_stream_chunk(&json) {
                                        return Some((Ok(chunk), es));
                                    }
                                    // Skip chunks without meaningful delta
                                }
                                Err(e) => {
                                    return Some((
                                        Err(SynwireError::from(OpenAIError::ResponseParse(
                                            e.to_string(),
                                        ))),
                                        es,
                                    ));
                                }
                            }
                        }
                        Some(Err(e)) => {
                            let msg: String = e.to_string();
                            return Some((
                                Err(SynwireError::from(OpenAIError::Http {
                                    status: None,
                                    message: msg,
                                })),
                                es,
                            ));
                        }
                        None => return None,
                    }
                }
            });

            Ok(Box::pin(chunk_stream) as BoxStream<'_, Result<ChatChunk, SynwireError>>)
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn model_type(&self) -> &str {
        "openai"
    }

    fn bind_tools(&self, tools: &[ToolSchema]) -> Result<Box<dyn BaseChatModel>, SynwireError> {
        let mut new_tools = self.tools.clone();
        new_tools.extend_from_slice(tools);
        Ok(Box::new(Self {
            base: BaseChatOpenAI {
                model: self.base.model.clone(),
                api_base: self.base.api_base.clone(),
                api_key: self.base.api_key.clone(),
                temperature: self.base.temperature,
                max_tokens: self.base.max_tokens,
                top_p: self.base.top_p,
                stop: self.base.stop.clone(),
                timeout: self.base.timeout,
                max_retries: self.base.max_retries,
                model_kwargs: self.base.model_kwargs.clone(),
                client: self.base.client.clone(),
                credential_provider: self.base.credential_provider.clone(),
            },
            tools: new_tools,
        }))
    }
}

/// Parses a non-streaming chat completion response.
fn parse_chat_response(json: &Value) -> Result<ChatResult, SynwireError> {
    let choices = json["choices"]
        .as_array()
        .ok_or_else(|| OpenAIError::ResponseParse("missing choices".into()))?;

    let choice = choices
        .first()
        .ok_or_else(|| OpenAIError::ResponseParse("empty choices".into()))?;

    let message_obj = &choice["message"];
    let content = message_obj["content"].as_str().unwrap_or("").to_string();

    // Parse tool calls if present
    let tool_calls = parse_tool_calls(message_obj);

    // Parse usage
    let usage = json.get("usage").map(|u| UsageMetadata {
        input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0),
        output_tokens: u["completion_tokens"].as_u64().unwrap_or(0),
        total_tokens: u["total_tokens"].as_u64().unwrap_or(0),
        ..Default::default()
    });

    let message = Message::AI {
        id: json.get("id").and_then(|v| v.as_str()).map(String::from),
        name: None,
        content: MessageContent::Text(content),
        tool_calls,
        invalid_tool_calls: Vec::new(),
        usage,
        response_metadata: None,
        additional_kwargs: HashMap::new(),
    };

    Ok(ChatResult {
        message,
        generation_info: None,
        cost: None,
    })
}

/// Parses tool calls from an `OpenAI` message object.
fn parse_tool_calls(message_obj: &Value) -> Vec<ToolCall> {
    message_obj["tool_calls"]
        .as_array()
        .map(|tcs| {
            tcs.iter()
                .filter_map(|tc| {
                    let id = tc["id"].as_str()?.to_string();
                    let name = tc["function"]["name"].as_str()?.to_string();
                    let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let arguments: HashMap<String, Value> =
                        serde_json::from_str(args_str).unwrap_or_default();
                    Some(ToolCall {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parses a streaming chunk from `OpenAI` SSE data.
fn parse_stream_chunk(json: &Value) -> Option<ChatChunk> {
    let choices = json["choices"].as_array()?;
    let choice = choices.first()?;
    let delta = &choice["delta"];

    let content = delta["content"].as_str().map(String::from);
    let finish_reason = choice["finish_reason"].as_str().map(String::from);

    let tool_calls = delta["tool_calls"]
        .as_array()
        .map(|tcs| {
            tcs.iter()
                .filter_map(|tc| {
                    let raw_index = tc["index"].as_u64()?;
                    let index = usize::try_from(raw_index).ok()?;
                    Some(ToolCallChunk {
                        index,
                        id: tc["id"].as_str().map(String::from),
                        name: tc["function"]["name"].as_str().map(String::from),
                        arguments: tc["function"]["arguments"].as_str().map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse usage if present (OpenAI includes in last chunk with stream_options)
    let usage = json.get("usage").and_then(|u| {
        Some(UsageMetadata {
            input_tokens: u["prompt_tokens"].as_u64()?,
            output_tokens: u["completion_tokens"].as_u64()?,
            total_tokens: u["total_tokens"].as_u64()?,
            ..Default::default()
        })
    });

    Some(ChatChunk {
        delta_content: content,
        delta_tool_calls: tool_calls,
        finish_reason,
        usage,
    })
}

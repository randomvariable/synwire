//! `ChatOllama` implementation.
//!
//! Provides a chat model backed by a local Ollama server. Supports both
//! non-streaming and streaming (NDJSON) invocations.

use crate::error::OllamaError;
use futures_util::StreamExt as _;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use synwire_core::credentials::CredentialProvider;
use synwire_core::error::SynwireError;
use synwire_core::language_models::traits::BaseChatModel;
use synwire_core::language_models::{ChatChunk, ChatResult};
use synwire_core::messages::{Message, MessageContent, UsageMetadata};
use synwire_core::runnables::RunnableConfig;
use synwire_core::tools::ToolSchema;
use synwire_core::{BoxFuture, BoxStream};

/// Default Ollama server base URL.
pub(crate) const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Default model name.
const DEFAULT_MODEL: &str = "llama3.2";

/// Default request timeout.
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Ollama chat model provider.
///
/// Sends requests to a local (or remote) Ollama server via the `/api/chat`
/// endpoint.
///
/// # Example
///
/// ```no_run
/// use synwire_llm_ollama::ChatOllama;
///
/// let model = ChatOllama::builder()
///     .model("llama3.2")
///     .build()
///     .unwrap();
/// ```
pub struct ChatOllama {
    model: String,
    base_url: String,
    temperature: Option<f32>,
    top_k: Option<u32>,
    top_p: Option<f32>,
    num_predict: Option<u32>,
    timeout: Duration,
    client: reqwest::Client,
    credential_provider: Option<Arc<dyn CredentialProvider>>,
}

impl std::fmt::Debug for ChatOllama {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatOllama")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("temperature", &self.temperature)
            .field("top_k", &self.top_k)
            .field("top_p", &self.top_p)
            .field("num_predict", &self.num_predict)
            .field("timeout", &self.timeout)
            .field(
                "credential_provider",
                &self.credential_provider.as_ref().map(|_| "..."),
            )
            .finish_non_exhaustive()
    }
}

impl ChatOllama {
    /// Returns a builder for constructing a [`ChatOllama`] instance.
    pub fn builder() -> ChatOllamaBuilder {
        ChatOllamaBuilder {
            model: DEFAULT_MODEL.into(),
            base_url: DEFAULT_BASE_URL.into(),
            temperature: None,
            top_k: None,
            top_p: None,
            num_predict: None,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            credential_provider: None,
        }
    }

    /// Returns the chat API URL.
    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }

    /// Builds the Ollama API request body.
    fn build_request_body(&self, messages: &[Message], stream: bool) -> Value {
        let mut obj = serde_json::Map::new();
        let _ = obj.insert("model".into(), serde_json::json!(self.model));
        let _ = obj.insert(
            "messages".into(),
            serde_json::json!(Self::convert_messages(messages)),
        );
        let _ = obj.insert("stream".into(), serde_json::json!(stream));

        let mut options = serde_json::Map::new();
        if let Some(temp) = self.temperature {
            let _ = options.insert("temperature".into(), serde_json::json!(temp));
        }
        if let Some(top_k) = self.top_k {
            let _ = options.insert("top_k".into(), serde_json::json!(top_k));
        }
        if let Some(top_p) = self.top_p {
            let _ = options.insert("top_p".into(), serde_json::json!(top_p));
        }
        if let Some(num_predict) = self.num_predict {
            let _ = options.insert("num_predict".into(), serde_json::json!(num_predict));
        }
        if !options.is_empty() {
            let _ = obj.insert("options".into(), Value::Object(options));
        }

        Value::Object(obj)
    }

    /// Converts Synwire messages to Ollama API format.
    fn convert_messages(messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.message_type() {
                    "human" => "user",
                    "ai" => "assistant",
                    "system" => "system",
                    other => other,
                };
                serde_json::json!({
                    "role": role,
                    "content": msg.content().as_text(),
                })
            })
            .collect()
    }

    /// Builds default request headers (no auth).
    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        let _ = headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    /// Builds request headers including an authorization header from the
    /// given key.
    fn headers_with_key(api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let _ = headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            let _ = headers.insert(AUTHORIZATION, val);
        }
        headers
    }

    /// Returns `true` if the given HTTP status indicates an authentication failure.
    const fn is_auth_failure(status: u16) -> bool {
        status == 401 || status == 403
    }

    /// Sends a POST request with optional credential-based auth retry.
    async fn send_with_auth_retry(
        &self,
        url: &str,
        body: &Value,
    ) -> Result<reqwest::Response, SynwireError> {
        // Determine initial headers.
        let headers = if let Some(ref provider) = self.credential_provider {
            let cred = provider.get_credential().await?;
            Self::headers_with_key(cred.expose())
        } else {
            Self::default_headers()
        };

        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| OllamaError::Http {
                status: e.status().map(|s| s.as_u16()),
                message: e.to_string(),
            })?;

        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let status_code = status.as_u16();
        let error_text = response.text().await.unwrap_or_default();

        // On 401/403 with a credential provider, refresh and retry once.
        if Self::is_auth_failure(status_code)
            && let Some(ref provider) = self.credential_provider
        {
            let refreshed = provider.refresh_credential().await?;
            let retry_headers = Self::headers_with_key(refreshed.expose());
            let retry_response = self
                .client
                .post(url)
                .headers(retry_headers)
                .json(body)
                .send()
                .await
                .map_err(|e| OllamaError::Http {
                    status: e.status().map(|s| s.as_u16()),
                    message: e.to_string(),
                })?;

            let retry_status = retry_response.status();
            if retry_status.is_success() {
                return Ok(retry_response);
            }

            let retry_error = retry_response.text().await.unwrap_or_default();
            return Err(OllamaError::Http {
                status: Some(retry_status.as_u16()),
                message: retry_error,
            }
            .into());
        }

        Err(OllamaError::Http {
            status: Some(status_code),
            message: error_text,
        }
        .into())
    }
}

/// Builder for [`ChatOllama`].
pub struct ChatOllamaBuilder {
    model: String,
    base_url: String,
    temperature: Option<f32>,
    top_k: Option<u32>,
    top_p: Option<f32>,
    num_predict: Option<u32>,
    timeout: Duration,
    credential_provider: Option<Arc<dyn CredentialProvider>>,
}

impl std::fmt::Debug for ChatOllamaBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatOllamaBuilder")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("temperature", &self.temperature)
            .field("top_k", &self.top_k)
            .field("top_p", &self.top_p)
            .field("num_predict", &self.num_predict)
            .field("timeout", &self.timeout)
            .field(
                "credential_provider",
                &self.credential_provider.as_ref().map(|_| "..."),
            )
            .finish_non_exhaustive()
    }
}

impl ChatOllamaBuilder {
    /// Sets the model name (e.g., `"llama3.2"`, `"mistral"`).
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets the Ollama server base URL.
    ///
    /// Defaults to `http://localhost:11434`.
    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Sets the temperature sampling parameter.
    #[must_use]
    pub const fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Sets the top-k sampling parameter.
    #[must_use]
    pub const fn top_k(mut self, k: u32) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Sets the top-p (nucleus) sampling parameter.
    #[must_use]
    pub const fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Sets the maximum number of tokens to predict.
    #[must_use]
    pub const fn num_predict(mut self, n: u32) -> Self {
        self.num_predict = Some(n);
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets a credential provider for dynamic credential refresh on 401/403.
    ///
    /// When set, the provider supplies bearer tokens for authenticated Ollama
    /// servers. On authentication failure, the credential is refreshed and
    /// the request retried once.
    #[must_use]
    pub fn credential_provider(mut self, provider: Arc<dyn CredentialProvider>) -> Self {
        self.credential_provider = Some(provider);
        self
    }

    /// Builds the [`ChatOllama`] instance.
    pub fn build(self) -> Result<ChatOllama, SynwireError> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| OllamaError::Http {
                status: None,
                message: e.to_string(),
            })?;

        Ok(ChatOllama {
            model: self.model,
            base_url: self.base_url,
            temperature: self.temperature,
            top_k: self.top_k,
            top_p: self.top_p,
            num_predict: self.num_predict,
            timeout: self.timeout,
            client,
            credential_provider: self.credential_provider,
        })
    }
}

impl BaseChatModel for ChatOllama {
    fn invoke<'a>(
        &'a self,
        messages: &'a [Message],
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<ChatResult, SynwireError>> {
        Box::pin(async move {
            let body = self.build_request_body(messages, false);
            let url = self.chat_url();

            let response = self.send_with_auth_retry(&url, &body).await?;

            let response_json: Value = response
                .json()
                .await
                .map_err(|e| OllamaError::ResponseParse(e.to_string()))?;

            parse_chat_response(&response_json)
        })
    }

    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, SynwireError>>, SynwireError>> {
        Box::pin(async move {
            let body = self.build_request_body(messages, true);
            let url = self.chat_url();

            let response = self.send_with_auth_retry(&url, &body).await?;

            let byte_stream = response.bytes_stream();

            // Ollama streams NDJSON: one JSON object per line.
            // We accumulate bytes, split on newlines, and parse each line.
            let chunk_stream = futures_util::stream::unfold(
                (byte_stream, Vec::<u8>::new()),
                |(mut stream, mut buffer)| async move {
                    loop {
                        // Check if the buffer already contains a complete line.
                        if let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                            let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                            let line = String::from_utf8_lossy(&line_bytes);
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            match serde_json::from_str::<Value>(trimmed) {
                                Ok(json) => {
                                    let done = json["done"].as_bool().unwrap_or(false);
                                    if let Some(chunk) = parse_stream_chunk(&json) {
                                        return Some((Ok(chunk), (stream, buffer)));
                                    }
                                    if done {
                                        return None;
                                    }
                                    continue;
                                }
                                Err(e) => {
                                    return Some((
                                        Err(SynwireError::from(OllamaError::ResponseParse(
                                            e.to_string(),
                                        ))),
                                        (stream, buffer),
                                    ));
                                }
                            }
                        }

                        // Read more data from the stream.
                        match stream.next().await {
                            Some(Ok(bytes)) => {
                                buffer.extend_from_slice(&bytes);
                            }
                            Some(Err(e)) => {
                                return Some((
                                    Err(SynwireError::from(OllamaError::Http {
                                        status: None,
                                        message: e.to_string(),
                                    })),
                                    (stream, buffer),
                                ));
                            }
                            None => {
                                // Stream ended; process any remaining data in buffer.
                                if buffer.is_empty() {
                                    return None;
                                }
                                let remaining = String::from_utf8_lossy(&buffer).trim().to_string();
                                buffer.clear();
                                if remaining.is_empty() {
                                    return None;
                                }
                                match serde_json::from_str::<Value>(&remaining) {
                                    Ok(json) => {
                                        if let Some(chunk) = parse_stream_chunk(&json) {
                                            return Some((Ok(chunk), (stream, buffer)));
                                        }
                                        return None;
                                    }
                                    Err(e) => {
                                        return Some((
                                            Err(SynwireError::from(OllamaError::ResponseParse(
                                                e.to_string(),
                                            ))),
                                            (stream, buffer),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                },
            );

            Ok(Box::pin(chunk_stream) as BoxStream<'_, Result<ChatChunk, SynwireError>>)
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn model_type(&self) -> &str {
        "ollama"
    }

    fn bind_tools(&self, _tools: &[ToolSchema]) -> Result<Box<dyn BaseChatModel>, SynwireError> {
        Err(SynwireError::Prompt {
            message: "bind_tools is not yet supported by Ollama provider".into(),
        })
    }
}

/// Parses a non-streaming Ollama `/api/chat` response.
fn parse_chat_response(json: &Value) -> Result<ChatResult, SynwireError> {
    let message_obj = json
        .get("message")
        .ok_or_else(|| OllamaError::ResponseParse("missing 'message' field".into()))?;

    let content = message_obj["content"].as_str().unwrap_or("").to_string();

    // Extract total_duration as generation info.
    let generation_info = json.get("total_duration").and_then(|d| {
        d.as_u64().map(|dur| {
            let mut info = HashMap::new();
            let _ = info.insert("total_duration_ns".into(), serde_json::json!(dur));
            info
        })
    });

    // Ollama doesn't provide token usage in the standard response by default,
    // but some versions include eval_count / prompt_eval_count.
    let usage = parse_usage(json);

    let message = Message::AI {
        id: None,
        name: None,
        content: MessageContent::Text(content),
        tool_calls: Vec::new(),
        invalid_tool_calls: Vec::new(),
        usage,
        response_metadata: None,
        additional_kwargs: HashMap::new(),
    };

    Ok(ChatResult {
        message,
        generation_info,
        cost: None,
    })
}

/// Parses a streaming NDJSON chunk from Ollama.
fn parse_stream_chunk(json: &Value) -> Option<ChatChunk> {
    let message_obj = json.get("message")?;
    let content = message_obj["content"].as_str().map(String::from);
    let done = json["done"].as_bool().unwrap_or(false);
    let finish_reason = if done { Some("stop".into()) } else { None };

    // Usage may be present in the final chunk.
    let usage = if done { parse_usage(json) } else { None };

    Some(ChatChunk {
        delta_content: content,
        delta_tool_calls: Vec::new(),
        finish_reason,
        usage,
    })
}

/// Extracts token usage from an Ollama response if available.
fn parse_usage(json: &Value) -> Option<UsageMetadata> {
    let prompt_tokens = json.get("prompt_eval_count").and_then(Value::as_u64);
    let output_tokens = json.get("eval_count").and_then(Value::as_u64);

    // Only build usage if at least one field is present.
    if prompt_tokens.is_some() || output_tokens.is_some() {
        let input = prompt_tokens.unwrap_or(0);
        let output = output_tokens.unwrap_or(0);
        Some(UsageMetadata {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
            ..Default::default()
        })
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults_to_localhost() {
        let builder = ChatOllama::builder();
        assert_eq!(builder.base_url, "http://localhost:11434");
        assert_eq!(builder.model, "llama3.2");
    }

    #[test]
    fn builder_sets_fields() {
        let builder = ChatOllama::builder()
            .model("mistral")
            .base_url("http://myserver:11434")
            .temperature(0.5)
            .top_k(40)
            .top_p(0.9)
            .num_predict(256);
        assert_eq!(builder.model, "mistral");
        assert_eq!(builder.base_url, "http://myserver:11434");
        assert_eq!(builder.temperature, Some(0.5));
        assert_eq!(builder.top_k, Some(40));
        assert_eq!(builder.top_p, Some(0.9));
        assert_eq!(builder.num_predict, Some(256));
    }

    #[test]
    fn builder_builds_successfully() {
        let model = ChatOllama::builder().build();
        assert!(model.is_ok());
    }

    #[test]
    fn model_type_is_ollama() {
        let model = ChatOllama::builder().build().unwrap();
        assert_eq!(model.model_type(), "ollama");
    }

    #[test]
    fn parse_chat_response_valid() {
        let json = serde_json::json!({
            "model": "llama3.2",
            "message": {"role": "assistant", "content": "Hello there!"},
            "done": true,
            "total_duration": 1_234_567_890_u64,
            "prompt_eval_count": 10,
            "eval_count": 5
        });

        let result = parse_chat_response(&json).unwrap();
        match &result.message {
            Message::AI {
                content: MessageContent::Text(text),
                usage,
                ..
            } => {
                assert_eq!(text, "Hello there!");
                let u = usage.as_ref().unwrap();
                assert_eq!(u.input_tokens, 10);
                assert_eq!(u.output_tokens, 5);
                assert_eq!(u.total_tokens, 15);
            }
            other => {
                panic!("expected AI message, got: {other:?}");
            }
        }
        assert!(result.generation_info.is_some());
    }

    #[test]
    fn parse_chat_response_missing_message() {
        let json = serde_json::json!({"done": true});
        let result = parse_chat_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_stream_chunk_with_content() {
        let json = serde_json::json!({
            "model": "llama3.2",
            "message": {"role": "assistant", "content": "Hi"},
            "done": false
        });
        let chunk = parse_stream_chunk(&json).unwrap();
        assert_eq!(chunk.delta_content.as_deref(), Some("Hi"));
        assert!(chunk.finish_reason.is_none());
    }

    #[test]
    fn parse_stream_chunk_done() {
        let json = serde_json::json!({
            "model": "llama3.2",
            "message": {"role": "assistant", "content": ""},
            "done": true,
            "prompt_eval_count": 8,
            "eval_count": 12
        });
        let chunk = parse_stream_chunk(&json).unwrap();
        assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
        let u = chunk.usage.as_ref().unwrap();
        assert_eq!(u.input_tokens, 8);
        assert_eq!(u.output_tokens, 12);
    }

    #[test]
    fn parse_stream_chunk_no_message_returns_none() {
        let json = serde_json::json!({"done": false});
        assert!(parse_stream_chunk(&json).is_none());
    }

    #[test]
    fn convert_messages_maps_roles() {
        let msgs = vec![
            Message::system("Be helpful"),
            Message::human("Hello"),
            Message::ai("Hi"),
        ];
        let converted = ChatOllama::convert_messages(&msgs);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0]["role"], "system");
        assert_eq!(converted[1]["role"], "user");
        assert_eq!(converted[2]["role"], "assistant");
    }

    #[test]
    fn request_body_includes_options_when_set() {
        let model = ChatOllama::builder()
            .temperature(0.7)
            .top_k(40)
            .build()
            .unwrap();
        let msgs = vec![Message::human("Hi")];
        let body = model.build_request_body(&msgs, false);
        assert_eq!(body["stream"], false);
        let temp = body["options"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 1e-5, "temperature was {temp}");
        assert_eq!(body["options"]["top_k"], 40);
    }

    #[test]
    fn request_body_omits_options_when_none() {
        let model = ChatOllama::builder().build().unwrap();
        let msgs = vec![Message::human("Hi")];
        let body = model.build_request_body(&msgs, false);
        assert!(body.get("options").is_none());
    }
}

//! Shared base type for `OpenAI`-compatible providers.

use crate::error::OpenAIError;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use synwire_core::credentials::CredentialProvider;

/// Shared base configuration for `OpenAI`-compatible providers.
pub struct BaseChatOpenAI {
    /// Model name (e.g., "gpt-4o").
    pub(crate) model: String,
    /// API base URL.
    pub(crate) api_base: String,
    /// API key.
    pub(crate) api_key: String,
    /// Temperature parameter.
    pub(crate) temperature: Option<f32>,
    /// Maximum tokens to generate.
    pub(crate) max_tokens: Option<u32>,
    /// Top-p sampling parameter.
    pub(crate) top_p: Option<f32>,
    /// Stop sequences.
    pub(crate) stop: Option<Vec<String>>,
    /// Request timeout.
    pub(crate) timeout: Duration,
    /// Maximum retries.
    pub(crate) max_retries: u32,
    /// Additional model kwargs.
    pub(crate) model_kwargs: HashMap<String, Value>,
    /// HTTP client.
    pub(crate) client: reqwest::Client,
    /// Optional credential provider for dynamic credential refresh.
    pub(crate) credential_provider: Option<Arc<dyn CredentialProvider>>,
}

impl std::fmt::Debug for BaseChatOpenAI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseChatOpenAI")
            .field("model", &self.model)
            .field("api_base", &self.api_base)
            .field("api_key", &"***")
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("top_p", &self.top_p)
            .field("stop", &self.stop)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field(
                "credential_provider",
                &self.credential_provider.as_ref().map(|_| "..."),
            )
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for BaseChatOpenAIBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseChatOpenAIBuilder")
            .field("model", &self.model)
            .field("api_base", &self.api_base)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("api_key_env", &self.api_key_env)
            .finish_non_exhaustive()
    }
}

/// Builder for [`BaseChatOpenAI`].
pub struct BaseChatOpenAIBuilder {
    model: String,
    api_base: String,
    api_key: Option<String>,
    api_key_env: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    timeout: Duration,
    max_retries: u32,
    model_kwargs: HashMap<String, Value>,
    credential_provider: Option<Arc<dyn CredentialProvider>>,
}

impl BaseChatOpenAIBuilder {
    /// Creates a new builder with defaults.
    pub fn new() -> Self {
        Self {
            model: "gpt-4o".into(),
            api_base: "https://api.openai.com/v1".into(),
            api_key: None,
            api_key_env: "OPENAI_API_KEY".into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            timeout: Duration::from_secs(60),
            max_retries: 2,
            model_kwargs: HashMap::new(),
            credential_provider: None,
        }
    }

    /// Sets the model name.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets the API base URL.
    #[must_use]
    pub fn api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self
    }

    /// Sets the API key directly.
    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets the environment variable name for the API key.
    #[must_use]
    pub fn api_key_env(mut self, env_var: impl Into<String>) -> Self {
        self.api_key_env = env_var.into();
        self
    }

    /// Sets the temperature parameter.
    #[must_use]
    pub const fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Sets the maximum tokens to generate.
    #[must_use]
    pub const fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Sets the top-p sampling parameter.
    #[must_use]
    pub const fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Sets the stop sequences.
    #[must_use]
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the maximum number of retries.
    #[must_use]
    pub const fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Sets a credential provider for dynamic credential refresh on 401/403.
    ///
    /// When set, the provider is used to obtain the API key at request time
    /// and to refresh it on authentication failures.
    #[must_use]
    pub fn credential_provider(mut self, provider: Arc<dyn CredentialProvider>) -> Self {
        self.credential_provider = Some(provider);
        self
    }

    /// Builds the [`BaseChatOpenAI`] instance.
    pub fn build(self) -> Result<BaseChatOpenAI, OpenAIError> {
        let api_key = self
            .api_key
            .or_else(|| std::env::var(&self.api_key_env).ok())
            .ok_or_else(|| {
                OpenAIError::Configuration(format!(
                    "API key not provided and {} not set",
                    self.api_key_env
                ))
            })?;

        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| OpenAIError::Http {
                status: None,
                message: e.to_string(),
            })?;

        Ok(BaseChatOpenAI {
            model: self.model,
            api_base: self.api_base,
            api_key,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: self.top_p,
            stop: self.stop,
            timeout: self.timeout,
            max_retries: self.max_retries,
            model_kwargs: self.model_kwargs,
            client,
            credential_provider: self.credential_provider,
        })
    }
}

impl Default for BaseChatOpenAIBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseChatOpenAI {
    /// Builds the request body for the chat completions endpoint.
    pub(crate) fn build_request_body(
        &self,
        messages: &[synwire_core::messages::Message],
        stream: bool,
        tools: Option<&[synwire_core::tools::ToolSchema]>,
    ) -> Value {
        let mut obj = serde_json::Map::new();
        let _ = obj.insert("model".into(), serde_json::json!(self.model));
        let _ = obj.insert(
            "messages".into(),
            serde_json::json!(Self::convert_messages(messages)),
        );
        let _ = obj.insert("stream".into(), serde_json::json!(stream));

        if let Some(temp) = self.temperature {
            let _ = obj.insert("temperature".into(), serde_json::json!(temp));
        }
        if let Some(max) = self.max_tokens {
            let _ = obj.insert("max_tokens".into(), serde_json::json!(max));
        }
        if let Some(top_p) = self.top_p {
            let _ = obj.insert("top_p".into(), serde_json::json!(top_p));
        }
        if let Some(ref stop) = self.stop {
            let _ = obj.insert("stop".into(), serde_json::json!(stop));
        }
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tool_defs: Vec<Value> = tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters,
                            }
                        })
                    })
                    .collect();
                let _ = obj.insert("tools".into(), serde_json::json!(tool_defs));
            }
        }

        // Merge additional kwargs
        for (k, v) in &self.model_kwargs {
            let _ = obj.insert(k.clone(), v.clone());
        }

        Value::Object(obj)
    }

    /// Converts Synwire messages to `OpenAI` API format.
    fn convert_messages(messages: &[synwire_core::messages::Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.message_type() {
                    "human" => "user",
                    "ai" => "assistant",
                    "system" => "system",
                    "tool" => "tool",
                    other => other,
                };
                let mut map = serde_json::Map::new();
                let _ = map.insert("role".into(), serde_json::json!(role));
                let _ = map.insert("content".into(), serde_json::json!(msg.content().as_text()));

                // Add tool_call_id for tool messages
                if let synwire_core::messages::Message::Tool { tool_call_id, .. } = msg {
                    let _ = map.insert("tool_call_id".into(), serde_json::json!(tool_call_id));
                }

                // Add tool_calls for AI messages
                if let synwire_core::messages::Message::AI { tool_calls, .. } = msg {
                    if !tool_calls.is_empty() {
                        let tc: Vec<Value> = tool_calls
                            .iter()
                            .map(|tc| {
                                let args_json =
                                    serde_json::to_string(&tc.arguments).unwrap_or_default();
                                serde_json::json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.name,
                                        "arguments": args_json,
                                    }
                                })
                            })
                            .collect();
                        let _ = map.insert("tool_calls".into(), serde_json::json!(tc));
                    }
                }

                Value::Object(map)
            })
            .collect()
    }

    /// Returns default headers for API requests.
    pub(crate) fn default_headers(&self) -> HeaderMap {
        Self::headers_with_key(&self.api_key)
    }

    /// Returns headers using the given API key.
    pub(crate) fn headers_with_key(api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let _ = headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            let _ = headers.insert(AUTHORIZATION, val);
        }
        headers
    }

    /// Returns `true` if the given HTTP status is an authentication failure.
    pub(crate) const fn is_auth_failure(status: u16) -> bool {
        status == 401 || status == 403
    }

    /// Returns the chat completions URL.
    pub(crate) fn completions_url(&self) -> String {
        format!("{}/chat/completions", self.api_base)
    }

    /// Sends a POST request to the given URL with the given body, retrying
    /// once on 401/403 if a [`CredentialProvider`] is configured.
    ///
    /// Returns the successful [`reqwest::Response`].
    pub(crate) async fn send_with_auth_retry(
        &self,
        url: &str,
        body: &Value,
    ) -> Result<reqwest::Response, synwire_core::error::SynwireError> {
        let headers = self.default_headers();
        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| OpenAIError::Http {
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
        if Self::is_auth_failure(status_code) {
            if let Some(ref provider) = self.credential_provider {
                let refreshed = provider.refresh_credential().await?;
                let retry_headers = Self::headers_with_key(refreshed.expose());
                let retry_response = self
                    .client
                    .post(url)
                    .headers(retry_headers)
                    .json(body)
                    .send()
                    .await
                    .map_err(|e| OpenAIError::Http {
                        status: e.status().map(|s| s.as_u16()),
                        message: e.to_string(),
                    })?;

                let retry_status = retry_response.status();
                if retry_status.is_success() {
                    return Ok(retry_response);
                }

                let retry_error = retry_response.text().await.unwrap_or_default();
                return Err(OpenAIError::Http {
                    status: Some(retry_status.as_u16()),
                    message: retry_error,
                }
                .into());
            }
        }

        Err(OpenAIError::Http {
            status: Some(status_code),
            message: error_text,
        }
        .into())
    }
}

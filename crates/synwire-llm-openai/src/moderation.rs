//! `OpenAI` content moderation middleware.

use synwire_core::BoxFuture;
use synwire_core::error::SynwireError;
use synwire_core::runnables::RunnableConfig;
use synwire_core::runnables::core::RunnableCore;

use crate::error::OpenAIError;

/// Middleware that checks content against the `OpenAI` Moderation API.
///
/// When used in a chain, it examines the input text and rejects it
/// if the moderation endpoint flags it as harmful.
///
/// # Examples
///
/// ```no_run
/// use synwire_llm_openai::moderation::OpenAIModerationMiddleware;
///
/// let middleware = OpenAIModerationMiddleware::new(
///     "https://api.openai.com/v1",
///     "sk-test-key",
/// );
/// let runnable = middleware.as_runnable();
/// ```
pub struct OpenAIModerationMiddleware {
    api_base: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIModerationMiddleware {
    /// Creates a new moderation middleware.
    pub fn new(api_base: &str, api_key: &str) -> Self {
        Self {
            api_base: api_base.to_owned(),
            api_key: api_key.to_owned(),
            client: reqwest::Client::new(),
        }
    }

    /// Returns this middleware as a [`RunnableCore`] for use in chains.
    ///
    /// The runnable expects a JSON string input (or a JSON object with a
    /// `"text"` field) and passes it through if safe, or returns an error
    /// if flagged.
    pub const fn as_runnable(self) -> ModerationRunnable {
        ModerationRunnable { middleware: self }
    }
}

/// A [`RunnableCore`] that performs content moderation.
pub struct ModerationRunnable {
    middleware: OpenAIModerationMiddleware,
}

impl RunnableCore for ModerationRunnable {
    fn invoke<'a>(
        &'a self,
        input: serde_json::Value,
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<serde_json::Value, SynwireError>> {
        Box::pin(async move {
            let text = extract_text(&input)?;

            let url = format!("{}/moderations", self.middleware.api_base);
            let body = serde_json::json!({ "input": text });

            let response = self
                .middleware
                .client
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.middleware.api_key),
                )
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    SynwireError::from(OpenAIError::Http {
                        status: e.status().map(|s| s.as_u16()),
                        message: e.to_string(),
                    })
                })?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(SynwireError::from(OpenAIError::Http {
                    status: Some(status.as_u16()),
                    message: error_text,
                }));
            }

            let resp: serde_json::Value = response
                .json()
                .await
                .map_err(|e| SynwireError::from(OpenAIError::ResponseParse(e.to_string())))?;

            // Check if any result is flagged
            let flagged = resp
                .get("results")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|results| {
                    results.iter().any(|r| {
                        r.get("flagged")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                    })
                });

            if flagged {
                return Err(SynwireError::Prompt {
                    message: "Content was flagged by the moderation API".into(),
                });
            }

            // Pass through the original input
            Ok(input)
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "OpenAIModerationMiddleware"
    }
}

/// Extracts text from input for moderation.
fn extract_text(input: &serde_json::Value) -> Result<String, SynwireError> {
    if let Some(s) = input.as_str() {
        return Ok(s.to_owned());
    }
    if let Some(obj) = input.as_object() {
        if let Some(text) = obj.get("text").and_then(serde_json::Value::as_str) {
            return Ok(text.to_owned());
        }
    }
    Err(SynwireError::Prompt {
        message: "moderation input must be a string or object with a \"text\" field".into(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_from_string() {
        let input = serde_json::json!("hello");
        let text = extract_text(&input).unwrap();
        assert_eq!(text, "hello");
    }

    #[test]
    fn extract_text_from_object() {
        let input = serde_json::json!({"text": "hello world"});
        let text = extract_text(&input).unwrap();
        assert_eq!(text, "hello world");
    }

    #[test]
    fn extract_text_rejects_number() {
        let input = serde_json::json!(42);
        let result = extract_text(&input);
        assert!(result.is_err());
    }

    #[test]
    fn middleware_creates_runnable() {
        let middleware = OpenAIModerationMiddleware::new("https://api.openai.com/v1", "sk-test");
        let runnable = middleware.as_runnable();
        assert_eq!(runnable.name(), "OpenAIModerationMiddleware");
    }
}

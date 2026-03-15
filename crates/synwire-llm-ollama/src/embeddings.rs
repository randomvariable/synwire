//! Ollama embeddings provider.
//!
//! Uses the Ollama `/api/embed` endpoint for text embeddings.

use crate::error::OllamaError;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use synwire_core::BoxFuture;
use synwire_core::credentials::CredentialProvider;
use synwire_core::embeddings::Embeddings;
use synwire_core::error::{EmbeddingError, SynwireError};

/// Default embedding model.
const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";

/// Ollama embeddings model.
///
/// Implements [`Embeddings`] by sending requests to the Ollama `/api/embed`
/// endpoint.
///
/// # Example
///
/// ```no_run
/// use synwire_llm_ollama::OllamaEmbeddings;
///
/// let embeddings = OllamaEmbeddings::builder()
///     .model("nomic-embed-text")
///     .build()
///     .unwrap();
/// ```
pub struct OllamaEmbeddings {
    model: String,
    base_url: String,
    client: reqwest::Client,
    credential_provider: Option<Arc<dyn CredentialProvider>>,
}

impl std::fmt::Debug for OllamaEmbeddings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OllamaEmbeddings")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field(
                "credential_provider",
                &self.credential_provider.as_ref().map(|_| "..."),
            )
            .finish_non_exhaustive()
    }
}

impl OllamaEmbeddings {
    /// Returns a builder for constructing an [`OllamaEmbeddings`] instance.
    pub fn builder() -> OllamaEmbeddingsBuilder {
        OllamaEmbeddingsBuilder {
            model: DEFAULT_EMBED_MODEL.into(),
            base_url: super::chat::DEFAULT_BASE_URL.to_owned(),
            timeout: Duration::from_secs(super::chat::DEFAULT_TIMEOUT_SECS),
            credential_provider: None,
        }
    }

    /// Returns the embed API URL.
    fn embed_url(&self) -> String {
        format!("{}/api/embed", self.base_url)
    }

    /// Builds default request headers (no auth).
    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        let _ = headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    /// Builds headers with bearer auth.
    fn headers_with_key(api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let _ = headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            let _ = headers.insert(AUTHORIZATION, val);
        }
        headers
    }

    /// Sends the embedding request, with optional credential retry.
    async fn embed_request(&self, input: &[String]) -> Result<Vec<Vec<f32>>, SynwireError> {
        let body = serde_json::json!({
            "model": self.model,
            "input": input,
        });

        let url = self.embed_url();

        let headers = if let Some(ref provider) = self.credential_provider {
            let cred = provider.get_credential().await?;
            Self::headers_with_key(cred.expose())
        } else {
            Self::default_headers()
        };

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| OllamaError::Http {
                status: e.status().map(|s| s.as_u16()),
                message: e.to_string(),
            })?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let error_text = response.text().await.unwrap_or_default();

            // On auth failure with credential provider, refresh and retry once.
            if status_code == 401 || status_code == 403 {
                if let Some(ref provider) = self.credential_provider {
                    let refreshed = provider.refresh_credential().await?;
                    let retry_headers = Self::headers_with_key(refreshed.expose());
                    let retry_response = self
                        .client
                        .post(&url)
                        .headers(retry_headers)
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| OllamaError::Http {
                            status: e.status().map(|s| s.as_u16()),
                            message: e.to_string(),
                        })?;

                    let retry_status = retry_response.status();
                    if retry_status.is_success() {
                        let json: Value =
                            retry_response
                                .json()
                                .await
                                .map_err(|e| EmbeddingError::Failed {
                                    message: e.to_string(),
                                })?;
                        return parse_embedding_response(&json);
                    }

                    let retry_status_code = retry_status.as_u16();
                    let retry_error = retry_response.text().await.unwrap_or_default();
                    return Err(OllamaError::Http {
                        status: Some(retry_status_code),
                        message: retry_error,
                    }
                    .into());
                }
            }

            return Err(OllamaError::Http {
                status: Some(status_code),
                message: error_text,
            }
            .into());
        }

        let json: Value = response.json().await.map_err(|e| EmbeddingError::Failed {
            message: e.to_string(),
        })?;

        parse_embedding_response(&json)
    }
}

/// Builder for [`OllamaEmbeddings`].
pub struct OllamaEmbeddingsBuilder {
    model: String,
    base_url: String,
    timeout: Duration,
    credential_provider: Option<Arc<dyn CredentialProvider>>,
}

impl std::fmt::Debug for OllamaEmbeddingsBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OllamaEmbeddingsBuilder")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field(
                "credential_provider",
                &self.credential_provider.as_ref().map(|_| "..."),
            )
            .finish_non_exhaustive()
    }
}

impl OllamaEmbeddingsBuilder {
    /// Sets the embedding model name.
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

    /// Sets the request timeout.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets a credential provider for dynamic credential refresh.
    #[must_use]
    pub fn credential_provider(mut self, provider: Arc<dyn CredentialProvider>) -> Self {
        self.credential_provider = Some(provider);
        self
    }

    /// Builds the [`OllamaEmbeddings`] instance.
    pub fn build(self) -> Result<OllamaEmbeddings, SynwireError> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| OllamaError::Http {
                status: None,
                message: e.to_string(),
            })?;

        Ok(OllamaEmbeddings {
            model: self.model,
            base_url: self.base_url,
            client,
            credential_provider: self.credential_provider,
        })
    }
}

impl Embeddings for OllamaEmbeddings {
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, SynwireError>> {
        Box::pin(async move { self.embed_request(texts).await })
    }

    fn embed_query<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Vec<f32>, SynwireError>> {
        Box::pin(async move {
            let texts = vec![text.to_string()];
            let mut results = self.embed_request(&texts).await?;
            results.pop().ok_or_else(|| {
                SynwireError::from(EmbeddingError::Failed {
                    message: "empty response from Ollama embeddings API".into(),
                })
            })
        })
    }
}

/// Parses the Ollama `/api/embed` response.
fn parse_embedding_response(json: &Value) -> Result<Vec<Vec<f32>>, SynwireError> {
    let embeddings_arr = json["embeddings"]
        .as_array()
        .ok_or_else(|| EmbeddingError::Failed {
            message: "missing 'embeddings' in response".into(),
        })?;

    let mut embeddings = Vec::with_capacity(embeddings_arr.len());

    for item in embeddings_arr {
        let arr = item.as_array().ok_or_else(|| EmbeddingError::Failed {
            message: "expected array in embeddings".into(),
        })?;

        #[allow(clippy::cast_possible_truncation)]
        let vec: Vec<f32> = arr
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        embeddings.push(vec);
    }

    Ok(embeddings)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let builder = OllamaEmbeddings::builder();
        assert_eq!(builder.model, "nomic-embed-text");
        assert_eq!(builder.base_url, "http://localhost:11434");
    }

    #[test]
    fn builder_sets_model() {
        let builder = OllamaEmbeddings::builder().model("mxbai-embed-large");
        assert_eq!(builder.model, "mxbai-embed-large");
    }

    #[test]
    fn builder_builds_successfully() {
        let result = OllamaEmbeddings::builder().build();
        assert!(result.is_ok());
    }

    #[test]
    fn parse_embedding_response_valid() {
        let json = serde_json::json!({
            "model": "nomic-embed-text",
            "embeddings": [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]
        });
        let result = parse_embedding_response(&json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 3);
        assert!((result[0][0] - 0.1).abs() < 1e-5);
        assert!((result[1][0] - 0.4).abs() < 1e-5);
    }

    #[test]
    fn parse_embedding_response_missing_field() {
        let json = serde_json::json!({"model": "test"});
        let result = parse_embedding_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_embedding_response_empty() {
        let json = serde_json::json!({
            "model": "test",
            "embeddings": []
        });
        let result = parse_embedding_response(&json).unwrap();
        assert!(result.is_empty());
    }
}

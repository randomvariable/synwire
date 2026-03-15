//! `OpenAI` embeddings provider.

use crate::base::{BaseChatOpenAI, BaseChatOpenAIBuilder};
use serde_json::Value;
use std::sync::Arc;
use synwire_core::BoxFuture;
use synwire_core::embeddings::Embeddings;
use synwire_core::error::{EmbeddingError, SynwireError};

/// `OpenAI` embeddings model.
///
/// Implements [`Embeddings`] by sending requests to the `OpenAI` embeddings API
/// (`/v1/embeddings`).
///
/// # Example
///
/// ```no_run
/// use synwire_llm_openai::OpenAIEmbeddings;
///
/// let embeddings = OpenAIEmbeddings::builder()
///     .model("text-embedding-3-small")
///     .api_key("sk-...")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct OpenAIEmbeddings {
    base: BaseChatOpenAI,
    model: String,
}

impl OpenAIEmbeddings {
    /// Returns a builder for constructing an [`OpenAIEmbeddings`] instance.
    pub fn builder() -> OpenAIEmbeddingsBuilder {
        OpenAIEmbeddingsBuilder {
            base: BaseChatOpenAIBuilder::new(),
            model: "text-embedding-3-small".into(),
        }
    }

    /// Returns the embeddings API URL.
    fn embeddings_url(&self) -> String {
        format!("{}/embeddings", self.base.api_base)
    }

    /// Sends an embedding request and parses the response.
    async fn embed_request(&self, input: &[String]) -> Result<Vec<Vec<f32>>, SynwireError> {
        let body = serde_json::json!({
            "model": self.model,
            "input": input,
        });

        let response = self
            .base
            .send_with_auth_retry(&self.embeddings_url(), &body)
            .await?;

        let json: Value = response.json().await.map_err(|e| {
            SynwireError::from(EmbeddingError::Failed {
                message: e.to_string(),
            })
        })?;

        parse_embedding_response(&json)
    }
}

/// Builder for [`OpenAIEmbeddings`].
#[derive(Debug)]
pub struct OpenAIEmbeddingsBuilder {
    base: BaseChatOpenAIBuilder,
    model: String,
}

impl OpenAIEmbeddingsBuilder {
    /// Sets the embedding model name.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
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

    /// Sets the API key environment variable name.
    #[must_use]
    pub fn api_key_env(mut self, env_var: impl Into<String>) -> Self {
        self.base = self.base.api_key_env(env_var);
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

    /// Sets a credential provider for dynamic credential refresh.
    #[must_use]
    pub fn credential_provider(
        mut self,
        provider: Arc<dyn synwire_core::credentials::CredentialProvider>,
    ) -> Self {
        self.base = self.base.credential_provider(provider);
        self
    }

    /// Builds the [`OpenAIEmbeddings`] instance.
    pub fn build(self) -> Result<OpenAIEmbeddings, SynwireError> {
        let base = self.base.build()?;
        Ok(OpenAIEmbeddings {
            base,
            model: self.model,
        })
    }
}

impl Embeddings for OpenAIEmbeddings {
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
                    message: "empty response from embeddings API".into(),
                })
            })
        })
    }
}

/// Parses the `OpenAI` embeddings API response.
fn parse_embedding_response(json: &Value) -> Result<Vec<Vec<f32>>, SynwireError> {
    let data = json["data"]
        .as_array()
        .ok_or_else(|| EmbeddingError::Failed {
            message: "missing 'data' in response".into(),
        })?;

    let mut embeddings: Vec<(u64, Vec<f32>)> = Vec::with_capacity(data.len());

    for item in data {
        let index = item["index"]
            .as_u64()
            .ok_or_else(|| EmbeddingError::Failed {
                message: "missing 'index' in embedding item".into(),
            })?;

        let embedding_arr = item["embedding"]
            .as_array()
            .ok_or_else(|| EmbeddingError::Failed {
                message: "missing 'embedding' in response item".into(),
            })?;

        #[allow(clippy::cast_possible_truncation)]
        let vec: Vec<f32> = embedding_arr
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        embeddings.push((index, vec));
    }

    // Sort by index to match input order
    embeddings.sort_by_key(|(idx, _)| *idx);

    Ok(embeddings.into_iter().map(|(_, v)| v).collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_embedding_response_valid() {
        let json = serde_json::json!({
            "data": [
                {"index": 0, "embedding": [0.1, 0.2, 0.3]},
                {"index": 1, "embedding": [0.4, 0.5, 0.6]},
            ],
            "usage": {"prompt_tokens": 10, "total_tokens": 10}
        });

        let result = parse_embedding_response(&json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 3);
        assert!((result[0][0] - 0.1).abs() < 1e-5);
        assert!((result[1][0] - 0.4).abs() < 1e-5);
    }

    #[test]
    fn parse_embedding_response_out_of_order() {
        let json = serde_json::json!({
            "data": [
                {"index": 1, "embedding": [0.4, 0.5]},
                {"index": 0, "embedding": [0.1, 0.2]},
            ]
        });

        let result = parse_embedding_response(&json).unwrap();
        // Should be sorted by index
        assert!((result[0][0] - 0.1).abs() < 1e-5);
        assert!((result[1][0] - 0.4).abs() < 1e-5);
    }

    #[test]
    fn parse_embedding_response_missing_data() {
        let json = serde_json::json!({"error": "bad request"});
        let result = parse_embedding_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn builder_sets_model() {
        // Just verify the builder compiles and sets model name correctly.
        let builder = OpenAIEmbeddings::builder().model("text-embedding-ada-002");
        assert_eq!(builder.model, "text-embedding-ada-002");
    }
}

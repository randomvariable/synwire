//! [`LocalEmbeddings`] — synchronous ONNX embedding model behind `spawn_blocking`.

use std::sync::Arc;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use synwire_core::{
    BoxFuture,
    embeddings::Embeddings,
    error::{EmbeddingError, SynwireError},
};
use tracing::debug;

/// Error type returned by [`LocalEmbeddings::new`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LocalEmbeddingsError {
    /// Failed to initialise the embedding model (download or ONNX load failure).
    #[error("failed to initialise embedding model: {0}")]
    Init(#[from] anyhow::Error),
}

/// Local text embedding using BAAI/bge-small-en-v1.5 (384 dimensions).
///
/// The underlying fastembed model is loaded synchronously on construction; all
/// subsequent [`Embeddings`] calls run the inference on the blocking thread pool
/// via [`tokio::task::spawn_blocking`].
///
/// Models are cached in the fastembed default cache directory on disk, so the
/// first call triggers a one-time download.
pub struct LocalEmbeddings {
    model: Arc<TextEmbedding>,
}

impl LocalEmbeddings {
    /// Create a new `LocalEmbeddings` backed by BAAI/bge-small-en-v1.5.
    ///
    /// Downloads the model on first call if not already cached locally.
    ///
    /// # Errors
    ///
    /// Returns [`LocalEmbeddingsError::Init`] when the model cannot be loaded or
    /// downloaded.
    pub fn new() -> Result<Self, LocalEmbeddingsError> {
        debug!("loading BAAI/bge-small-en-v1.5 embedding model");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(false),
        )?;
        debug!("embedding model ready");
        Ok(Self {
            model: Arc::new(model),
        })
    }
}

impl Embeddings for LocalEmbeddings {
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, SynwireError>> {
        let model = Arc::clone(&self.model);
        let owned: Vec<String> = texts.to_vec();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || model.embed(owned, None))
                .await
                .map_err(|_| {
                    SynwireError::Embedding(EmbeddingError::Failed {
                        message: "embedding task panicked".into(),
                    })
                })?
                .map_err(|e| {
                    SynwireError::Embedding(EmbeddingError::Failed {
                        message: e.to_string(),
                    })
                })
        })
    }

    fn embed_query<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Vec<f32>, SynwireError>> {
        let model = Arc::clone(&self.model);
        let owned = text.to_owned();
        Box::pin(async move {
            let mut results = tokio::task::spawn_blocking(move || model.embed(vec![owned], None))
                .await
                .map_err(|_| {
                    SynwireError::Embedding(EmbeddingError::Failed {
                        message: "embedding task panicked".into(),
                    })
                })?
                .map_err(|e| {
                    SynwireError::Embedding(EmbeddingError::Failed {
                        message: e.to_string(),
                    })
                })?;
            results.pop().ok_or_else(|| {
                SynwireError::Embedding(EmbeddingError::Failed {
                    message: "model returned no embeddings".into(),
                })
            })
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// Verify construction succeeds (skipped in CI when no network/cache).
    #[test]
    #[ignore = "requires model download"]
    fn construction_succeeds() {
        let _ = LocalEmbeddings::new().expect("model should load");
    }
}

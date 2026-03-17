//! [`LocalReranker`] — synchronous ONNX reranking model behind `spawn_blocking`.

use std::sync::Arc;

use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use synwire_core::{
    BoxFuture,
    documents::Document,
    error::{EmbeddingError, SynwireError},
    rerankers::Reranker,
};
use tracing::debug;

/// Error type returned by [`LocalReranker::new`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LocalRerankerError {
    /// Failed to initialise the reranking model (download or ONNX load failure).
    #[error("failed to initialise reranking model: {0}")]
    Init(#[from] anyhow::Error),
}

/// Local document reranker using BAAI/bge-reranker-base.
///
/// The underlying fastembed model is loaded synchronously on construction; all
/// subsequent [`Reranker`] calls run the cross-encoder inference on the blocking
/// thread pool via [`tokio::task::spawn_blocking`].
///
/// Models are cached in the fastembed default cache directory on disk, so the
/// first call triggers a one-time download.
pub struct LocalReranker {
    model: Arc<TextRerank>,
}

impl LocalReranker {
    /// Create a new `LocalReranker` backed by BAAI/bge-reranker-base.
    ///
    /// Downloads the model on first call if not already cached locally.
    ///
    /// # Errors
    ///
    /// Returns [`LocalRerankerError::Init`] when the model cannot be loaded or
    /// downloaded.
    pub fn new() -> Result<Self, LocalRerankerError> {
        debug!("loading BAAI/bge-reranker-base reranking model");
        let model = TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::BGERerankerBase)
                .with_show_download_progress(false),
        )?;
        debug!("reranking model ready");
        Ok(Self {
            model: Arc::new(model),
        })
    }
}

impl Reranker for LocalReranker {
    fn rerank<'a>(
        &'a self,
        query: &'a str,
        documents: &'a [Document],
        top_n: usize,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>> {
        let model = Arc::clone(&self.model);
        let query_owned = query.to_owned();
        // Collect page content strings for fastembed; indices map back to `documents`.
        let texts: Vec<String> = documents.iter().map(|d| d.page_content.clone()).collect();
        let documents_owned: Vec<Document> = documents.to_vec();

        Box::pin(async move {
            let mut results = tokio::task::spawn_blocking(move || {
                // return_documents=false: we do our own index mapping.
                model.rerank(query_owned, texts, false, None)
            })
            .await
            .map_err(|_| {
                SynwireError::Embedding(EmbeddingError::Failed {
                    message: "reranking task panicked".into(),
                })
            })?
            .map_err(|e| {
                SynwireError::Embedding(EmbeddingError::Failed {
                    message: e.to_string(),
                })
            })?;

            // Sort descending by score so the highest-relevance document is first.
            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let reranked: Vec<Document> = results
                .into_iter()
                .take(top_n)
                .filter_map(|r| documents_owned.get(r.index).cloned())
                .collect();

            Ok(reranked)
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
        let _ = LocalReranker::new().expect("model should load");
    }
}

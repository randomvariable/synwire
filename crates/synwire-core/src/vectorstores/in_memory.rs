//! In-memory vector store for testing and small datasets.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::BoxFuture;
use crate::documents::Document;
use crate::embeddings::Embeddings;
use crate::error::{SynwireError, VectorStoreError};

use super::mmr::cosine_similarity;
use super::traits::VectorStore;

/// In-memory vector store backed by a `HashMap`.
///
/// Uses brute-force cosine similarity for retrieval. Suitable for testing
/// and small datasets. Not recommended for production use at scale.
///
/// # Examples
///
/// ```
/// use synwire_core::vectorstores::InMemoryVectorStore;
/// let store = InMemoryVectorStore::new();
/// ```
pub struct InMemoryVectorStore {
    /// Maps document ID to (document, embedding vector).
    documents: RwLock<HashMap<String, (Document, Vec<f32>)>>,
    /// Expected embedding dimensions. Set on first insertion.
    expected_dimensions: RwLock<Option<usize>>,
}

impl InMemoryVectorStore {
    /// Creates a new empty in-memory vector store.
    pub fn new() -> Self {
        Self {
            documents: RwLock::new(HashMap::new()),
            expected_dimensions: RwLock::new(None),
        }
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to map a `RwLock` poison error to a `VectorStoreError`.
#[allow(clippy::needless_pass_by_value)]
fn lock_error<T>(err: std::sync::PoisonError<T>) -> VectorStoreError {
    VectorStoreError::Other {
        message: format!("lock poisoned: {err}"),
    }
}

#[allow(clippy::significant_drop_tightening)]
impl VectorStore for InMemoryVectorStore {
    fn add_documents<'a>(
        &'a self,
        documents: &'a [Document],
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<String>, SynwireError>> {
        Box::pin(async move {
            let texts: Vec<String> = documents.iter().map(|d| d.page_content.clone()).collect();
            let vectors = embeddings.embed_documents(&texts).await?;

            let mut ids = Vec::with_capacity(documents.len());

            // Scope the lock guards to drop them as early as possible.
            {
                let mut doc_store = self.documents.write().map_err(lock_error)?;
                let mut dims_guard = self.expected_dimensions.write().map_err(lock_error)?;

                for (doc, embedding_vec) in documents.iter().zip(vectors) {
                    // Check dimension consistency
                    if let Some(expected) = *dims_guard {
                        if embedding_vec.len() != expected {
                            return Err(VectorStoreError::DimensionMismatch {
                                expected,
                                actual: embedding_vec.len(),
                            }
                            .into());
                        }
                    } else {
                        *dims_guard = Some(embedding_vec.len());
                    }

                    let id = doc
                        .id
                        .clone()
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                    let mut stored_doc = doc.clone();
                    stored_doc.id = Some(id.clone());
                    let _ = doc_store.insert(id.clone(), (stored_doc, embedding_vec));
                    ids.push(id);
                }
            }

            Ok(ids)
        })
    }

    fn similarity_search<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>> {
        Box::pin(async move {
            let results = self
                .similarity_search_with_score(query, k, embeddings)
                .await?;
            Ok(results.into_iter().map(|(doc, _)| doc).collect())
        })
    }

    fn similarity_search_with_score<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<(Document, f32)>, SynwireError>> {
        Box::pin(async move {
            let query_vec = embeddings.embed_query(query).await?;

            let mut scored: Vec<(Document, f32)> = {
                let doc_store = self.documents.read().map_err(lock_error)?;
                doc_store
                    .values()
                    .map(|(doc, embedding_vec)| {
                        let sim = cosine_similarity(&query_vec, embedding_vec);
                        (doc.clone(), sim)
                    })
                    .collect()
            };

            // Sort descending by similarity
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(k);

            Ok(scored)
        })
    }

    fn delete<'a>(&'a self, ids: &'a [String]) -> BoxFuture<'a, Result<(), SynwireError>> {
        Box::pin(async move {
            let mut doc_store = self.documents.write().map_err(lock_error)?;
            for id in ids {
                let _ = doc_store.remove(id);
            }
            drop(doc_store);
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::embeddings::FakeEmbeddings;

    #[tokio::test]
    async fn add_documents_returns_ids() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);
        let docs = vec![Document::new("hello world"), Document::new("goodbye world")];
        let ids = store.add_documents(&docs, &embeddings).await.unwrap();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }

    #[tokio::test]
    async fn similarity_search_returns_ranked() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let docs = vec![
            Document::new("the cat sat on the mat"),
            Document::new("quantum mechanics and relativity"),
            Document::new("the cat played with yarn"),
        ];
        let _ = store.add_documents(&docs, &embeddings).await.unwrap();

        let results = store
            .similarity_search("cat and mat", 2, &embeddings)
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn empty_store_returns_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryVectorStore::new();
            let embeddings = FakeEmbeddings::new(32);
            let results = store
                .similarity_search("anything", 5, &embeddings)
                .await
                .unwrap();
            assert!(results.is_empty());
        });
    }

    #[tokio::test]
    async fn similarity_search_with_score_descending() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let docs = vec![
            Document::new("alpha"),
            Document::new("beta"),
            Document::new("gamma"),
        ];
        let _ = store.add_documents(&docs, &embeddings).await.unwrap();

        let results = store
            .similarity_search_with_score("alpha", 3, &embeddings)
            .await
            .unwrap();

        // Scores should be in descending order
        for window in results.windows(2) {
            assert!(window[0].1 >= window[1].1);
        }
    }

    #[tokio::test]
    async fn delete_removes_documents() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let docs = vec![Document::new("to be deleted")];
        let ids = store.add_documents(&docs, &embeddings).await.unwrap();

        store.delete(&ids).await.unwrap();

        let results = store
            .similarity_search("to be deleted", 5, &embeddings)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn preserves_document_ids() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let mut doc = Document::new("with id");
        doc.id = Some("my-custom-id".into());

        let ids = store.add_documents(&[doc], &embeddings).await.unwrap();
        assert_eq!(ids, vec!["my-custom-id"]);
    }
}

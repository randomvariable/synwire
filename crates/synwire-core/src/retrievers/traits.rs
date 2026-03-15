//! Retriever trait and types.

use crate::BoxFuture;
use crate::documents::Document;
use crate::embeddings::Embeddings;
use crate::error::SynwireError;
use crate::vectorstores::VectorStore;
use crate::vectorstores::mmr::maximal_marginal_relevance;

/// Trait for document retrievers.
///
/// A retriever takes a natural-language query and returns relevant documents.
/// This is the primary abstraction for retrieval-augmented generation (RAG).
pub trait Retriever: Send + Sync {
    /// Retrieve documents relevant to the query.
    fn get_relevant_documents<'a>(
        &'a self,
        query: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>>;
}

/// The similarity search strategy to use.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SearchType {
    /// Standard cosine similarity search.
    Similarity,
    /// Maximal Marginal Relevance search, balancing relevance and diversity.
    Mmr {
        /// Controls the relevance-diversity trade-off.
        /// `1.0` = pure relevance, `0.0` = maximum diversity.
        lambda: f32,
    },
}

/// The retrieval mode (dense, sparse, or hybrid).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RetrievalMode {
    /// Dense vector retrieval (default).
    Dense,
    /// Sparse keyword-based retrieval (e.g., BM25).
    Sparse,
    /// Hybrid retrieval combining dense and sparse.
    Hybrid {
        /// Weight for dense retrieval. `1.0` = pure dense, `0.0` = pure sparse.
        alpha: f32,
    },
}

/// A retriever backed by a [`VectorStore`] and [`Embeddings`] model.
///
/// Wraps a vector store to provide the [`Retriever`] interface with
/// configurable search type and retrieval mode.
pub struct VectorStoreRetriever {
    store: Box<dyn VectorStore>,
    embeddings: Box<dyn Embeddings>,
    k: usize,
    search_type: SearchType,
    retrieval_mode: RetrievalMode,
}

impl VectorStoreRetriever {
    /// Creates a new `VectorStoreRetriever`.
    pub fn new(
        store: Box<dyn VectorStore>,
        embeddings: Box<dyn Embeddings>,
        k: usize,
        search_type: SearchType,
        retrieval_mode: RetrievalMode,
    ) -> Self {
        Self {
            store,
            embeddings,
            k,
            search_type,
            retrieval_mode,
        }
    }
}

impl Retriever for VectorStoreRetriever {
    fn get_relevant_documents<'a>(
        &'a self,
        query: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>> {
        Box::pin(async move {
            // Reject unsupported modes
            match &self.retrieval_mode {
                RetrievalMode::Sparse => {
                    return Err(SynwireError::Other(
                        "sparse retrieval is not supported by VectorStoreRetriever".into(),
                    ));
                }
                RetrievalMode::Hybrid { .. } => {
                    return Err(SynwireError::Other(
                        "hybrid retrieval is not supported by VectorStoreRetriever".into(),
                    ));
                }
                RetrievalMode::Dense => {}
            }

            match &self.search_type {
                SearchType::Similarity => {
                    self.store
                        .similarity_search(query, self.k, self.embeddings.as_ref())
                        .await
                }
                SearchType::Mmr { lambda } => {
                    // Fetch more candidates than k for MMR re-ranking
                    let fetch_k = self.k * 4;
                    let candidates = self
                        .store
                        .similarity_search_with_score(query, fetch_k, self.embeddings.as_ref())
                        .await?;

                    if candidates.is_empty() {
                        return Ok(Vec::new());
                    }

                    let query_vec = self.embeddings.embed_query(query).await?;
                    let texts: Vec<String> = candidates
                        .iter()
                        .map(|(doc, _)| doc.page_content.clone())
                        .collect();
                    let candidate_embeddings = self.embeddings.embed_documents(&texts).await?;

                    let indices = maximal_marginal_relevance(
                        &query_vec,
                        &candidate_embeddings,
                        self.k,
                        *lambda,
                    );

                    Ok(indices
                        .into_iter()
                        .filter_map(|i| candidates.get(i).map(|(doc, _)| doc.clone()))
                        .collect())
                }
            }
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::embeddings::FakeEmbeddings;
    use crate::vectorstores::InMemoryVectorStore;

    #[tokio::test]
    async fn vector_store_retriever_wraps_store() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let docs = vec![
            Document::new("rust programming"),
            Document::new("python scripting"),
            Document::new("rust ownership model"),
        ];
        let _ = store.add_documents(&docs, &embeddings).await.unwrap();

        let retriever = VectorStoreRetriever::new(
            Box::new(store),
            Box::new(embeddings),
            2,
            SearchType::Similarity,
            RetrievalMode::Dense,
        );

        let results = retriever.get_relevant_documents("rust").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn vector_store_retriever_mmr_search() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let docs = vec![
            Document::new("alpha beta"),
            Document::new("alpha gamma"),
            Document::new("delta epsilon"),
        ];
        let _ = store.add_documents(&docs, &embeddings).await.unwrap();

        let retriever = VectorStoreRetriever::new(
            Box::new(store),
            Box::new(embeddings),
            2,
            SearchType::Mmr { lambda: 0.5 },
            RetrievalMode::Dense,
        );

        let results = retriever.get_relevant_documents("alpha").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn retriever_sparse_mode_returns_error() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let retriever = VectorStoreRetriever::new(
            Box::new(store),
            Box::new(embeddings),
            2,
            SearchType::Similarity,
            RetrievalMode::Sparse,
        );

        let result = retriever.get_relevant_documents("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn retriever_hybrid_mode_returns_error() {
        let store = InMemoryVectorStore::new();
        let embeddings = FakeEmbeddings::new(32);

        let retriever = VectorStoreRetriever::new(
            Box::new(store),
            Box::new(embeddings),
            2,
            SearchType::Similarity,
            RetrievalMode::Hybrid { alpha: 0.5 },
        );

        let result = retriever.get_relevant_documents("test").await;
        assert!(result.is_err());
    }
}

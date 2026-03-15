//! Trait definition for vector stores.

use crate::BoxFuture;
use crate::documents::Document;
use crate::embeddings::Embeddings;
use crate::error::SynwireError;

/// Trait for vector stores that persist and query document embeddings.
///
/// Implementations manage storage of documents alongside their embedding
/// vectors and support similarity-based retrieval.
///
/// # Cancel safety
///
/// [`add_documents`](Self::add_documents) is **not cancel-safe**: dropping
/// the future mid-write may leave a partial set of documents persisted.
/// [`similarity_search`](Self::similarity_search) and
/// [`similarity_search_with_score`](Self::similarity_search_with_score) are
/// cancel-safe for read-only stores, but dropping mid-query may waste
/// compute. [`delete`](Self::delete) is **not cancel-safe**: partial
/// deletions are possible.
pub trait VectorStore: Send + Sync {
    /// Add documents to the store, returning their assigned IDs.
    fn add_documents<'a>(
        &'a self,
        documents: &'a [Document],
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<String>, SynwireError>>;

    /// Retrieve the `k` most similar documents to the query.
    fn similarity_search<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>>;

    /// Retrieve the `k` most similar documents with their similarity scores.
    fn similarity_search_with_score<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<(Document, f32)>, SynwireError>>;

    /// Delete documents by their IDs.
    fn delete<'a>(&'a self, ids: &'a [String]) -> BoxFuture<'a, Result<(), SynwireError>>;
}

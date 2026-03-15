//! Reranker trait definition.

use crate::BoxFuture;
use crate::documents::Document;
use crate::error::SynwireError;

/// Trait for document rerankers.
///
/// A reranker takes a query and a set of candidate documents and returns
/// the top-n documents re-scored by relevance.
pub trait Reranker: Send + Sync {
    /// Rerank documents by relevance to the query, returning the top `top_n`.
    fn rerank<'a>(
        &'a self,
        query: &'a str,
        documents: &'a [Document],
        top_n: usize,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>>;
}

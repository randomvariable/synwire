//! Trait definition for text embedding models.

use crate::BoxFuture;
use crate::error::SynwireError;

/// Trait for text embedding models.
///
/// Implementors produce fixed-dimension floating-point vectors from text input,
/// suitable for similarity search, clustering, and retrieval-augmented generation.
///
/// # Cancel safety
///
/// The futures returned by [`embed_documents`](Self::embed_documents) and
/// [`embed_query`](Self::embed_query) are **not cancel-safe**. Dropping a
/// future mid-flight may leave partial results undelivered. Use
/// [`tokio::time::timeout`] for bounded waits and retry the full request
/// on timeout.
pub trait Embeddings: Send + Sync {
    /// Embed a list of texts, returning one vector per text.
    ///
    /// The returned outer `Vec` has the same length as `texts`, and each inner
    /// `Vec<f32>` is an embedding vector of consistent dimensionality.
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, SynwireError>>;

    /// Embed a single query text.
    ///
    /// Some providers use different models or parameters for queries versus
    /// documents; this method handles that distinction.
    fn embed_query<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Vec<f32>, SynwireError>>;
}

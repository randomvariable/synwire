//! Cache-backed embeddings that memoize results using moka.

use std::sync::Arc;

use moka::future::Cache;
use synwire_core::BoxFuture;
use synwire_core::embeddings::Embeddings;
use synwire_core::error::SynwireError;

/// Wraps an [`Embeddings`] implementation with an in-memory moka cache.
///
/// Repeated calls with the same text return cached vectors instead of
/// re-invoking the underlying embeddings model.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use synwire::cache::CacheBackedEmbeddings;
/// use synwire_core::embeddings::FakeEmbeddings;
///
/// let inner = Arc::new(FakeEmbeddings::new(64));
/// let cached = CacheBackedEmbeddings::new(inner, 1000);
/// ```
pub struct CacheBackedEmbeddings {
    inner: Arc<dyn Embeddings>,
    cache: Cache<String, Vec<f32>>,
}

impl CacheBackedEmbeddings {
    /// Creates a new cache-backed embeddings wrapper.
    ///
    /// `max_capacity` controls how many embedding vectors to cache.
    pub fn new(inner: Arc<dyn Embeddings>, max_capacity: u64) -> Self {
        Self {
            inner,
            cache: Cache::new(max_capacity),
        }
    }
}

impl Embeddings for CacheBackedEmbeddings {
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, SynwireError>> {
        Box::pin(async move {
            let mut results = Vec::with_capacity(texts.len());
            let mut uncached_indices = Vec::new();
            let mut uncached_texts = Vec::new();

            // Check cache for each text
            for (i, text) in texts.iter().enumerate() {
                if let Some(cached) = self.cache.get(text).await {
                    results.push(Some(cached));
                } else {
                    results.push(None);
                    uncached_indices.push(i);
                    uncached_texts.push(text.clone());
                }
            }

            // Embed uncached texts
            if !uncached_texts.is_empty() {
                let embedded = self.inner.embed_documents(&uncached_texts).await?;
                for (idx, vec) in uncached_indices.into_iter().zip(embedded) {
                    self.cache.insert(texts[idx].clone(), vec.clone()).await;
                    results[idx] = Some(vec);
                }
            }

            // All entries should be Some now; collect them
            Ok(results.into_iter().map(Option::unwrap_or_default).collect())
        })
    }

    fn embed_query<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Vec<f32>, SynwireError>> {
        Box::pin(async move {
            if let Some(cached) = self.cache.get(text).await {
                return Ok(cached);
            }
            let vec = self.inner.embed_query(text).await?;
            self.cache.insert(text.to_owned(), vec.clone()).await;
            Ok(vec)
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use synwire_core::embeddings::FakeEmbeddings;

    #[tokio::test]
    async fn cache_hit_returns_same_vector() {
        let inner = Arc::new(FakeEmbeddings::new(16));
        let cached = CacheBackedEmbeddings::new(inner, 100);

        let v1 = cached.embed_query("hello").await.unwrap();
        let v2 = cached.embed_query("hello").await.unwrap();
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn cache_miss_invokes_inner() {
        let inner = Arc::new(FakeEmbeddings::new(16));
        let cached = CacheBackedEmbeddings::new(inner, 100);

        let v1 = cached.embed_query("alpha").await.unwrap();
        let v2 = cached.embed_query("beta").await.unwrap();
        assert_ne!(v1, v2);
    }

    #[tokio::test]
    async fn embed_documents_caches_per_text() {
        let inner = Arc::new(FakeEmbeddings::new(8));
        let cached = CacheBackedEmbeddings::new(inner, 100);

        let texts = vec!["foo".into(), "bar".into()];
        let first = cached.embed_documents(&texts).await.unwrap();

        // Second call should hit cache
        let second = cached.embed_documents(&texts).await.unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn embed_documents_partial_cache() {
        let inner = Arc::new(FakeEmbeddings::new(8));
        let cached = CacheBackedEmbeddings::new(inner, 100);

        // Pre-populate cache with "foo"
        let _v = cached.embed_query("foo").await.unwrap();

        // Now embed ["foo", "bar"] -- "foo" from cache, "bar" fresh
        let texts = vec!["foo".into(), "bar".into()];
        let result = cached.embed_documents(&texts).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 8);
        assert_eq!(result[1].len(), 8);
    }
}

//! Deterministic fake embeddings for testing.

use crate::BoxFuture;
use crate::error::SynwireError;

use super::traits::Embeddings;

/// Deterministic fake embeddings for testing.
///
/// Generates vectors by hashing the input text. The same text always produces
/// the same embedding vector, making tests reproducible.
///
/// # Examples
///
/// ```
/// use synwire_core::embeddings::FakeEmbeddings;
/// let embeddings = FakeEmbeddings::new(128);
/// ```
pub struct FakeEmbeddings {
    dimensions: usize,
}

impl FakeEmbeddings {
    /// Creates a new `FakeEmbeddings` with the given vector dimensionality.
    pub const fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }

    /// Generates a deterministic embedding vector for a given text.
    fn embed_text(&self, text: &str) -> Vec<f32> {
        let mut vector = Vec::with_capacity(self.dimensions);
        for i in 0..self.dimensions {
            #[allow(clippy::cast_possible_truncation)]
            let hash = text.bytes().enumerate().fold(0u32, |acc, (j, b)| {
                acc.wrapping_add(
                    u32::from(b)
                        .wrapping_mul((j + 1) as u32)
                        .wrapping_mul((i + 1) as u32),
                )
            });
            // Map to [0, 1) range
            #[allow(clippy::cast_precision_loss)]
            let val = (hash % 10_000) as f32 / 10_000.0;
            vector.push(val);
        }

        // Normalize to unit length
        #[allow(clippy::cast_precision_loss)]
        let magnitude = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if magnitude > f32::EPSILON {
            for v in &mut vector {
                *v /= magnitude;
            }
        }

        vector
    }
}

impl Embeddings for FakeEmbeddings {
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, SynwireError>> {
        Box::pin(async move { Ok(texts.iter().map(|t| self.embed_text(t)).collect()) })
    }

    fn embed_query<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Vec<f32>, SynwireError>> {
        Box::pin(async move { Ok(self.embed_text(text)) })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_embeddings_returns_consistent_dimensions() {
        let embeddings = FakeEmbeddings::new(64);
        let texts = vec!["hello".into(), "world".into(), "foo bar".into()];
        let result = embeddings.embed_documents(&texts).await.unwrap();
        assert_eq!(result.len(), 3);
        for vec in &result {
            assert_eq!(vec.len(), 64);
        }
    }

    #[tokio::test]
    async fn embed_query_returns_single_vector() {
        let embeddings = FakeEmbeddings::new(32);
        let result = embeddings.embed_query("test query").await.unwrap();
        assert_eq!(result.len(), 32);
    }

    #[tokio::test]
    async fn fake_embeddings_are_deterministic() {
        let embeddings = FakeEmbeddings::new(16);
        let v1 = embeddings.embed_query("hello").await.unwrap();
        let v2 = embeddings.embed_query("hello").await.unwrap();
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn fake_embeddings_are_normalized() {
        let embeddings = FakeEmbeddings::new(64);
        let v = embeddings.embed_query("normalize me").await.unwrap();
        let magnitude: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-4, "magnitude = {magnitude}");
    }

    #[tokio::test]
    async fn different_texts_produce_different_embeddings() {
        let embeddings = FakeEmbeddings::new(16);
        let v1 = embeddings.embed_query("alpha").await.unwrap();
        let v2 = embeddings.embed_query("beta").await.unwrap();
        assert_ne!(v1, v2);
    }
}

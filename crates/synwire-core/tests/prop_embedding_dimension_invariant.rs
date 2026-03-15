//! Property test: Embedding vectors produced by `FakeEmbeddings` always have
//! correct dimensionality and are normalized.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_core::embeddings::{Embeddings, FakeEmbeddings};
use synwire_test_utils::embeddings::arb_embedding_dimension;

proptest! {
    /// FakeEmbeddings should always produce vectors of the configured dimension.
    #[test]
    fn embedding_dimension_matches(dim in arb_embedding_dimension()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let embeddings = FakeEmbeddings::new(dim);
            let result = embeddings.embed_query("test text").await.unwrap();
            assert_eq!(result.len(), dim, "expected {dim} dimensions, got {}", result.len());
        });
    }

    /// FakeEmbeddings should produce normalized vectors (magnitude ~1.0).
    #[test]
    fn embedding_is_normalized(dim in arb_embedding_dimension()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let embeddings = FakeEmbeddings::new(dim);
            let result = embeddings.embed_query("normalize check").await.unwrap();
            let magnitude: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!(
                (magnitude - 1.0).abs() < 1e-3,
                "magnitude should be ~1.0, got {magnitude}"
            );
        });
    }

    /// FakeEmbeddings should be deterministic: same input always gives same output.
    #[test]
    fn embedding_is_deterministic(dim in arb_embedding_dimension(), text in ".*") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let embeddings = FakeEmbeddings::new(dim);
            let v1 = embeddings.embed_query(&text).await.unwrap();
            let v2 = embeddings.embed_query(&text).await.unwrap();
            assert_eq!(v1, v2, "same text should produce same embedding");
        });
    }
}

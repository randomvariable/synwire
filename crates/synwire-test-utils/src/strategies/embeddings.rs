//! Proptest strategies for embedding vectors.

use proptest::prelude::*;

/// Strategy for generating a normalized embedding vector of given dimension.
pub fn arb_normalized_embedding(dim: usize) -> impl Strategy<Value = Vec<f32>> {
    prop::collection::vec(-1.0f32..1.0f32, dim).prop_map(|mut v| {
        let magnitude: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > f32::EPSILON {
            for x in &mut v {
                *x /= magnitude;
            }
        } else {
            // Degenerate case: set first element to 1.0
            if let Some(first) = v.first_mut() {
                *first = 1.0;
            }
        }
        v
    })
}

/// Strategy for generating a batch of normalized embeddings.
pub fn arb_embedding_batch(dim: usize, count: usize) -> impl Strategy<Value = Vec<Vec<f32>>> {
    prop::collection::vec(arb_normalized_embedding(dim), count)
}

/// Strategy for generating an embedding dimension in a reasonable range.
pub fn arb_embedding_dimension() -> impl Strategy<Value = usize> {
    prop_oneof![Just(32), Just(64), Just(128), Just(256), Just(512),]
}

//! Embedding traits and types.
//!
//! This module provides the [`Embeddings`] trait for text embedding models,
//! plus a [`FakeEmbeddings`] implementation for deterministic testing.

mod fake;
mod traits;

pub use fake::FakeEmbeddings;
pub use traits::Embeddings;

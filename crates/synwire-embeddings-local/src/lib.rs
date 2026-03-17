//! Local embedding and reranking for Synwire using fastembed-rs.
//!
//! Provides [`LocalEmbeddings`] (BAAI/bge-small-en-v1.5, 384 dims) and
//! [`LocalReranker`] (BAAI/bge-reranker-base) with automatic model download
//! on first use via Hugging Face Hub.
//!
//! Both types wrap fastembed's synchronous ONNX inference behind
//! `tokio::task::spawn_blocking`, keeping the async interface non-blocking.
//!
//! # Example
//!
//! ```no_run
//! use synwire_embeddings_local::{LocalEmbeddings, LocalReranker};
//! use synwire_core::embeddings::Embeddings;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = LocalEmbeddings::new()?;
//! let vecs = embedder.embed_query("hello world").await?;
//! assert_eq!(vecs.len(), 384);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod embeddings;
mod reranker;

pub use embeddings::{LocalEmbeddings, LocalEmbeddingsError};
pub use reranker::{LocalReranker, LocalRerankerError};

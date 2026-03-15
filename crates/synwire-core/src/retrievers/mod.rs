//! Retriever traits and types.
//!
//! This module provides the [`Retriever`] trait for document retrieval,
//! [`VectorStoreRetriever`] that wraps a vector store, and configuration
//! types [`SearchType`] and [`RetrievalMode`].

mod traits;

pub use traits::{RetrievalMode, Retriever, SearchType, VectorStoreRetriever};

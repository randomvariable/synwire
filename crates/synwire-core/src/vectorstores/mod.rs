//! Vector store traits and types.
//!
//! This module provides the [`VectorStore`] trait, [`MetadataFilter`] for
//! filtering results, [`InMemoryVectorStore`] for testing, and the
//! [`mmr`] module for Maximal Marginal Relevance selection.

mod filter;
mod in_memory;
pub mod mmr;
mod traits;

pub use filter::MetadataFilter;
pub use in_memory::InMemoryVectorStore;
pub use traits::VectorStore;

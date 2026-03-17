//! Semantic indexing pipeline for Synwire VFS providers.
//!
//! Orchestrates directory walking, AST-aware chunking, embedding, and vector
//! storage into a single [`SemanticIndex`] that VFS providers delegate to.

#![forbid(unsafe_code)]

mod cache;
mod config;
mod hashes;
mod index;
mod pipeline;
mod walker;
mod watcher;

pub mod xref;

#[cfg(feature = "code-graph")]
pub mod graph;

#[cfg(feature = "community-detection")]
pub mod community;

#[cfg(feature = "hybrid-search")]
mod bm25;

#[cfg(feature = "hybrid-search")]
mod hybrid;

pub use config::IndexConfig;
pub use index::SemanticIndex;
pub use index::StoreFactory;
pub use xref::{XrefDirection, XrefEdge, XrefGraph, rebuild_project_xrefs, xref_query};

#[cfg(feature = "hybrid-search")]
pub use bm25::{Bm25Error, Bm25Index};

#[cfg(feature = "hybrid-search")]
pub use hybrid::{HybridResult, HybridSearchConfig, hybrid_search};

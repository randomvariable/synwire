//! Dynamic call graph construction via LSP goto-definition.
//!
//! Builds a call graph on-demand by following goto-definition requests.

mod graph;

pub use graph::{CallNode, DynamicCallGraph};

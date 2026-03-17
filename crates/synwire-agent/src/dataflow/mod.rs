//! Dataflow retrieval: trace variable origins via tree-sitter heuristics.
//!
//! Identifies where a variable is defined and modified, using a combination
//! of tree-sitter pattern matching and simple backward-slice heuristics.

mod tracer;

pub use tracer::{DataflowHop, DataflowOrigin, DataflowTracer};

//! Types for the code dependency graph.

use serde::{Deserialize, Serialize};

/// Unique identifier for a graph node (file + symbol).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId {
    /// Relative file path within the project.
    pub file: String,
    /// Symbol name (e.g., function, struct, class). Empty for file-level nodes.
    pub symbol: String,
}

/// The kind of an edge between two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EdgeKind {
    /// Source calls the target function/method.
    Calls,
    /// Source file imports the target module/file.
    Imports,
    /// Source symbol is contained within the target symbol.
    Contains,
    /// Source type inherits from target type.
    Inherits,
    /// Source references target (generic reference).
    References,
}

/// A directed edge between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Origin node.
    pub from: NodeId,
    /// Destination node.
    pub to: NodeId,
    /// Type of relationship.
    pub kind: EdgeKind,
    /// Source line number where the reference occurs (0 if unknown).
    pub line: u32,
}

/// Query for traversing the graph.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GraphQuery {
    /// Starting node symbol (file or `file::symbol`).
    pub symbol: String,
    /// Maximum traversal depth.
    pub depth: u32,
    /// If true, follow incoming edges (callers); if false, outgoing (callees).
    pub incoming: bool,
}

/// Result of a graph query.
#[derive(Debug, Clone)]
pub struct GraphResult {
    /// Nodes reachable from the query root within the depth limit.
    pub nodes: Vec<NodeId>,
    /// Edges traversed.
    pub edges: Vec<Edge>,
}

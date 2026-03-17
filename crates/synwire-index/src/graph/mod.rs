//! Code dependency graph for the synwire semantic index.
//!
//! Provides a cross-file call/import/inherit graph with incremental updates
//! and multi-hop traversal.

pub mod builder;
pub mod query;
pub mod storage;
pub mod types;

pub use builder::GraphBuilder;
pub use query::graph_query;
pub use storage::GraphStorage;
pub use types::{Edge, EdgeKind, GraphQuery, GraphResult, NodeId};

/// High-level code dependency graph combining storage, building, and querying.
#[derive(Debug, Default)]
pub struct CodeGraph {
    storage: GraphStorage,
    builder: GraphBuilder,
}

impl CodeGraph {
    /// Create an empty code graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Index a source file, updating the graph incrementally.
    pub fn index_file(&mut self, relative_path: &str, content: &str) {
        self.builder
            .extract(relative_path, content, &mut self.storage);
    }

    /// Execute a graph query.
    #[must_use]
    pub fn query(&self, q: &GraphQuery) -> GraphResult {
        graph_query(&self.storage, q)
    }

    /// Total number of edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.storage.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_and_query_imports() {
        let mut g = CodeGraph::new();
        g.index_file("src/main.rs", "use std::collections;\nuse crate::utils;");
        let q = GraphQuery {
            symbol: "src/main.rs".to_owned(),
            depth: 1,
            incoming: false,
        };
        let result = g.query(&q);
        // Should find at least the root node
        assert!(!result.nodes.is_empty());
    }

    #[test]
    fn incremental_update_removes_old_edges() {
        let mut g = CodeGraph::new();
        g.index_file("a.rs", "use b;");
        let before = g.edge_count();
        // Re-index with different content
        g.index_file("a.rs", "");
        let after = g.edge_count();
        assert!(after < before || after == 0);
    }

    #[test]
    fn incoming_traversal() {
        let mut g = CodeGraph::new();
        g.index_file("a.rs", "use b;");
        let q = GraphQuery {
            symbol: "b".to_owned(),
            depth: 1,
            incoming: true,
        };
        let result = g.query(&q);
        // Should include a.rs as an incoming node (via import edge)
        assert!(!result.nodes.is_empty());
    }
}

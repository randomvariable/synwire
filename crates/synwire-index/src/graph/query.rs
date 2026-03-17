//! Graph query operations.

use crate::graph::{
    storage::GraphStorage,
    types::{GraphQuery, GraphResult, NodeId},
};

/// Execute a [`GraphQuery`] against `storage`.
#[must_use]
pub fn graph_query(storage: &GraphStorage, query: &GraphQuery) -> GraphResult {
    // Find the matching node — try exact file match first.
    let root = NodeId {
        file: query.symbol.clone(),
        symbol: String::new(),
    };
    let (nodes, edges) = if query.incoming {
        storage.incoming(&root, query.depth)
    } else {
        storage.outgoing(&root, query.depth)
    };
    GraphResult { nodes, edges }
}

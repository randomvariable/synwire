//! Graph send type for routing values to specific nodes.

use serde::{Deserialize, Serialize};

/// A directive to send a value to a specific node in the graph.
///
/// Named `GraphSend` to avoid conflict with [`std::marker::Send`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSend {
    /// The target node name.
    pub node: String,
    /// The value to send.
    pub arg: serde_json::Value,
}

impl GraphSend {
    /// Creates a new `GraphSend` targeting the given node with the given value.
    pub fn new(node: impl Into<String>, arg: serde_json::Value) -> Self {
        Self {
            node: node.into(),
            arg,
        }
    }
}

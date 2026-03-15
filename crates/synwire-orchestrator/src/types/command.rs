//! Command types for controlling graph execution flow.

use serde::{Deserialize, Serialize};

/// A command that can be issued during graph execution to alter control flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Command {
    /// Resume execution from an interrupt.
    Resume {
        /// The value to resume with.
        value: serde_json::Value,
    },
    /// Jump to a specific node.
    Goto {
        /// The target node name.
        node: String,
    },
    /// Update the graph state.
    Update {
        /// The state update to apply.
        update: serde_json::Value,
    },
}

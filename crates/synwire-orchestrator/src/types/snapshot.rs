//! State snapshot types for graph introspection.

use serde::{Deserialize, Serialize};

/// A snapshot of the graph state at a point in execution.
///
/// Used for checkpointing, debugging, and streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// The current state values.
    pub values: serde_json::Value,
    /// The next nodes to execute, if any.
    pub next: Vec<String>,
    /// Metadata about the snapshot.
    pub metadata: SnapshotMetadata,
}

/// Metadata associated with a state snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// The current step number.
    pub step: usize,
    /// The node that produced this snapshot, if any.
    pub source: Option<String>,
}

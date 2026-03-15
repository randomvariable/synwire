//! Node execution state types.

use serde::{Deserialize, Serialize};

/// The execution state of a node within a superstep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeState {
    /// The node is pending execution.
    Pending,
    /// The node is currently executing.
    Running,
    /// The node completed successfully.
    Completed,
    /// The node failed with an error.
    Failed,
    /// The node was skipped.
    Skipped,
}

/// Strategy for handling node errors during graph execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeErrorStrategy {
    /// Stop execution on the first error.
    #[default]
    Fail,
    /// Continue execution despite errors (best effort).
    Continue,
    /// Retry the failed node according to the retry policy.
    Retry,
}

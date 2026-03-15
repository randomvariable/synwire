//! Stream mode configuration for graph execution.

use serde::{Deserialize, Serialize};

/// Controls what data is emitted during streaming graph execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StreamMode {
    /// Stream complete state snapshots after each superstep.
    Values,
    /// Stream only the state updates (deltas) after each superstep.
    Updates,
    /// Stream debug information about execution.
    Debug,
    /// Stream individual messages as they are produced.
    Messages,
    /// Stream custom events emitted by nodes.
    Custom,
}

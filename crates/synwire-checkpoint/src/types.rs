//! Checkpoint types for persisting graph state.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A checkpoint of graph state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique checkpoint ID.
    pub id: String,
    /// Channel values at this checkpoint.
    pub channel_values: HashMap<String, serde_json::Value>,
    /// Channel versions for conflict detection.
    pub channel_versions: HashMap<String, ChannelVersion>,
    /// Pending writes not yet applied.
    pub pending_writes: Vec<PendingWrite>,
    /// Format version for migration support.
    pub format_version: String,
}

impl Checkpoint {
    /// Create a new checkpoint with the given ID and default values.
    pub fn new(id: String) -> Self {
        Self {
            id,
            channel_values: HashMap::new(),
            channel_versions: HashMap::new(),
            pending_writes: Vec::new(),
            format_version: "1.0".into(),
        }
    }
}

/// A version marker for a channel, used for conflict detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelVersion {
    /// The monotonically increasing version number.
    pub version: u64,
}

/// A pending write that has not yet been applied to the checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingWrite {
    /// The channel this write targets.
    pub channel: String,
    /// The value to write.
    pub value: serde_json::Value,
}

/// Metadata associated with a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// The source that created this checkpoint.
    pub source: CheckpointSource,
    /// The step number in the graph execution.
    pub step: i64,
    /// Writes that were applied at this step.
    pub writes: HashMap<String, serde_json::Value>,
    /// Parent checkpoint references.
    pub parents: HashMap<String, String>,
}

/// The source of a checkpoint creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CheckpointSource {
    /// Checkpoint created from initial input.
    Input,
    /// Checkpoint created during a loop iteration.
    Loop,
    /// Checkpoint created from an explicit update.
    Update,
}

/// A complete checkpoint tuple containing all associated data.
#[derive(Debug, Clone)]
pub struct CheckpointTuple {
    /// The configuration that identifies this checkpoint.
    pub config: CheckpointConfig,
    /// The checkpoint data.
    pub checkpoint: Checkpoint,
    /// Metadata about how this checkpoint was created.
    pub metadata: CheckpointMetadata,
    /// The parent checkpoint configuration, if any.
    pub parent_config: Option<CheckpointConfig>,
}

/// Configuration identifying a checkpoint within a thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// The thread ID this checkpoint belongs to.
    pub thread_id: String,
    /// The specific checkpoint ID, if targeting an exact checkpoint.
    pub checkpoint_id: Option<String>,
}

/// Errors that can occur during checkpoint operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CheckpointError {
    /// The requested checkpoint was not found.
    #[error("checkpoint not found")]
    NotFound,
    /// The serialized state exceeds the configured maximum size.
    #[error("state too large: {size} bytes exceeds max {max}")]
    StateTooLarge {
        /// The actual size of the serialized state.
        size: usize,
        /// The maximum allowed size.
        max: usize,
    },
    /// A serialization or deserialization error occurred.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// An error occurred in the underlying storage backend.
    #[error("storage error: {0}")]
    Storage(String),
}

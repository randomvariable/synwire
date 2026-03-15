//! Channel trait definitions for the Pregel engine.
//!
//! Channels are the primary mechanism for passing state between nodes in a
//! graph execution. Each channel stores a value that can be updated, read,
//! checkpointed, and restored.

use crate::error::GraphError;

/// A channel that stores and manages state for a single key in the graph.
///
/// Channels accumulate updates during a superstep and expose a current value
/// for downstream nodes to read.
pub trait BaseChannel: Send + Sync {
    /// Returns the key that identifies this channel.
    fn key(&self) -> &str;

    /// Applies a batch of values to this channel.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::MultipleValues`] if the channel does not support
    /// multiple values in a single superstep.
    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError>;

    /// Returns the current value of this channel, if any.
    fn get(&self) -> Option<&serde_json::Value>;

    /// Returns a JSON representation of this channel for checkpointing.
    fn checkpoint(&self) -> serde_json::Value;

    /// Restores the channel state from a checkpoint value.
    fn restore_checkpoint(&mut self, value: serde_json::Value);

    /// Consumes and returns the current value, resetting the channel.
    fn consume(&mut self) -> Option<serde_json::Value>;

    /// Returns `true` if the channel has a value available to read.
    fn is_available(&self) -> bool;
}

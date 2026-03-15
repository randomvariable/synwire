//! Core checkpoint saver trait.

use synwire_core::BoxFuture;

use crate::types::{
    Checkpoint, CheckpointConfig, CheckpointError, CheckpointMetadata, CheckpointTuple,
};

/// Trait for persisting and retrieving graph checkpoints.
///
/// Implementations must be thread-safe (`Send + Sync`) and return
/// boxed futures for async compatibility without the `async-trait` macro.
pub trait BaseCheckpointSaver: Send + Sync {
    /// Retrieve a single checkpoint tuple matching the given configuration.
    ///
    /// If `config.checkpoint_id` is `None`, returns the latest checkpoint
    /// for the given thread. Returns `Ok(None)` if no matching checkpoint exists.
    fn get_tuple<'a>(
        &'a self,
        config: &'a CheckpointConfig,
    ) -> BoxFuture<'a, Result<Option<CheckpointTuple>, CheckpointError>>;

    /// List checkpoint tuples for the given configuration.
    ///
    /// Returns checkpoints in reverse chronological order (newest first).
    /// If `limit` is `Some(n)`, returns at most `n` results.
    fn list<'a>(
        &'a self,
        config: &'a CheckpointConfig,
        limit: Option<usize>,
    ) -> BoxFuture<'a, Result<Vec<CheckpointTuple>, CheckpointError>>;

    /// Persist a checkpoint with its metadata.
    ///
    /// Returns the updated `CheckpointConfig` with the assigned checkpoint ID.
    fn put<'a>(
        &'a self,
        config: &'a CheckpointConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
    ) -> BoxFuture<'a, Result<CheckpointConfig, CheckpointError>>;
}

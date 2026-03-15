//! In-memory checkpoint saver implementation.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::base::BaseCheckpointSaver;
use crate::types::{
    Checkpoint, CheckpointConfig, CheckpointError, CheckpointMetadata, CheckpointTuple,
};

/// An in-memory checkpoint saver backed by a `RwLock<HashMap>`.
///
/// Stores checkpoints in memory keyed by thread ID. Suitable for
/// testing and short-lived processes. Data is lost when the process exits.
#[derive(Debug, Clone, Default)]
pub struct InMemoryCheckpointSaver {
    storage: Arc<RwLock<HashMap<String, Vec<CheckpointTuple>>>>,
}

impl InMemoryCheckpointSaver {
    /// Create a new, empty in-memory checkpoint saver.
    pub fn new() -> Self {
        Self::default()
    }
}

#[allow(clippy::significant_drop_tightening)]
impl BaseCheckpointSaver for InMemoryCheckpointSaver {
    fn get_tuple<'a>(
        &'a self,
        config: &'a CheckpointConfig,
    ) -> synwire_core::BoxFuture<'a, Result<Option<CheckpointTuple>, CheckpointError>> {
        Box::pin(async move {
            let storage = self.storage.read().await;
            let Some(tuples) = storage.get(&config.thread_id) else {
                return Ok(None);
            };
            Ok(config.checkpoint_id.as_ref().map_or_else(
                || tuples.last().cloned(),
                |checkpoint_id| {
                    tuples
                        .iter()
                        .find(|t| t.checkpoint.id == *checkpoint_id)
                        .cloned()
                },
            ))
        })
    }

    fn list<'a>(
        &'a self,
        config: &'a CheckpointConfig,
        limit: Option<usize>,
    ) -> synwire_core::BoxFuture<'a, Result<Vec<CheckpointTuple>, CheckpointError>> {
        Box::pin(async move {
            let storage = self.storage.read().await;
            let Some(tuples) = storage.get(&config.thread_id) else {
                return Ok(Vec::new());
            };
            let mut result: Vec<CheckpointTuple> = tuples.iter().rev().cloned().collect();
            if let Some(limit) = limit {
                result.truncate(limit);
            }
            Ok(result)
        })
    }

    fn put<'a>(
        &'a self,
        config: &'a CheckpointConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
    ) -> synwire_core::BoxFuture<'a, Result<CheckpointConfig, CheckpointError>> {
        Box::pin(async move {
            let new_config = CheckpointConfig {
                thread_id: config.thread_id.clone(),
                checkpoint_id: Some(checkpoint.id.clone()),
            };

            let mut storage = self.storage.write().await;
            let tuples = storage.entry(config.thread_id.clone()).or_default();
            let parent_config = tuples.last().map(|t| t.config.clone());

            tuples.push(CheckpointTuple {
                config: new_config.clone(),
                checkpoint,
                metadata,
                parent_config,
            });

            Ok(new_config)
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::CheckpointSource;
    use serde_json::json;

    fn make_checkpoint(id: &str, step: i64) -> (Checkpoint, CheckpointMetadata) {
        let mut cp = Checkpoint::new(id.to_owned());
        let _prev = cp.channel_values.insert("messages".into(), json!([]));
        let metadata = CheckpointMetadata {
            source: CheckpointSource::Loop,
            step,
            writes: HashMap::new(),
            parents: HashMap::new(),
        };
        (cp, metadata)
    }

    /// T216: `InMemoryCheckpointSaver` put and get round-trip.
    #[tokio::test]
    async fn put_and_get_round_trip() {
        let saver = InMemoryCheckpointSaver::new();
        let config = CheckpointConfig {
            thread_id: "thread-1".into(),
            checkpoint_id: None,
        };
        let (cp, meta) = make_checkpoint("cp-1", 0);
        let result_config = saver.put(&config, cp, meta).await.unwrap();
        assert_eq!(result_config.checkpoint_id.as_deref(), Some("cp-1"));

        // Get by thread_id (latest)
        let tuple = saver.get_tuple(&config).await.unwrap().unwrap();
        assert_eq!(tuple.checkpoint.id, "cp-1");
        assert_eq!(tuple.checkpoint.channel_values["messages"], json!([]));

        // Get by specific checkpoint_id
        let specific = CheckpointConfig {
            thread_id: "thread-1".into(),
            checkpoint_id: Some("cp-1".into()),
        };
        let tuple = saver.get_tuple(&specific).await.unwrap().unwrap();
        assert_eq!(tuple.checkpoint.id, "cp-1");

        // Get non-existent
        let missing = CheckpointConfig {
            thread_id: "no-such-thread".into(),
            checkpoint_id: None,
        };
        assert!(saver.get_tuple(&missing).await.unwrap().is_none());
    }

    /// T217: list returns in reverse chronological order.
    #[tokio::test]
    async fn list_returns_in_order() {
        let saver = InMemoryCheckpointSaver::new();
        let config = CheckpointConfig {
            thread_id: "thread-1".into(),
            checkpoint_id: None,
        };

        for i in 0..5 {
            let (cp, meta) = make_checkpoint(&format!("cp-{i}"), i64::from(i));
            let _cfg = saver.put(&config, cp, meta).await.unwrap();
        }

        // List all -- newest first
        let all = saver.list(&config, None).await.unwrap();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].checkpoint.id, "cp-4");
        assert_eq!(all[4].checkpoint.id, "cp-0");

        // List with limit
        let limited = saver.list(&config, Some(2)).await.unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].checkpoint.id, "cp-4");
        assert_eq!(limited[1].checkpoint.id, "cp-3");

        // Parent config should chain
        assert!(all[0].parent_config.is_some());
        assert_eq!(
            all[0]
                .parent_config
                .as_ref()
                .unwrap()
                .checkpoint_id
                .as_deref(),
            Some("cp-3")
        );
    }

    /// T223: `format_version` defaults to "1.0".
    #[tokio::test]
    async fn format_version_default() {
        let cp = Checkpoint::new("test".into());
        assert_eq!(cp.format_version, "1.0");
    }
}

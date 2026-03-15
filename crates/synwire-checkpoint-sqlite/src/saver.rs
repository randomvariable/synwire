//! SQLite-backed checkpoint saver implementation.

use std::path::Path;
use std::sync::Arc;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use synwire_checkpoint::base::BaseCheckpointSaver;
use synwire_checkpoint::types::{
    Checkpoint, CheckpointConfig, CheckpointError, CheckpointMetadata, CheckpointTuple,
};

use crate::schema::CREATE_CHECKPOINTS_TABLE;

/// Default maximum checkpoint size in bytes (16 MiB).
const DEFAULT_MAX_CHECKPOINT_SIZE: usize = 16 * 1024 * 1024;

/// SQLite-backed checkpoint saver.
///
/// Persists checkpoints to a `SQLite` database file with configurable
/// maximum checkpoint size. The database file is created with mode 0600
/// permissions on Unix systems.
#[derive(Debug, Clone)]
pub struct SqliteSaver {
    pool: Arc<Pool<SqliteConnectionManager>>,
    max_checkpoint_size: usize,
}

impl SqliteSaver {
    /// Create a new `SqliteSaver` at the given path.
    ///
    /// Creates the database file (with 0600 permissions on Unix) and
    /// initialises the schema if it does not already exist.
    ///
    /// # Errors
    ///
    /// Returns `CheckpointError::Storage` if the database cannot be opened
    /// or the schema cannot be created.
    pub fn new(path: &Path) -> Result<Self, CheckpointError> {
        Self::with_max_size(path, DEFAULT_MAX_CHECKPOINT_SIZE)
    }

    /// Create a new `SqliteSaver` with a custom maximum checkpoint size.
    ///
    /// # Errors
    ///
    /// Returns `CheckpointError::Storage` if the database cannot be opened
    /// or the schema cannot be created.
    pub fn with_max_size(path: &Path, max_checkpoint_size: usize) -> Result<Self, CheckpointError> {
        // Set file permissions to 0600 on Unix before opening.
        #[cfg(unix)]
        {
            if !path.exists() {
                // Create the file first so we can set permissions.
                let _file = std::fs::File::create(path)
                    .map_err(|e| CheckpointError::Storage(e.to_string()))?;
                Self::set_permissions_0600(path)?;
            }
        }

        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(4)
            .build(manager)
            .map_err(|e| CheckpointError::Storage(e.to_string()))?;

        // Initialise schema.
        let conn = pool
            .get()
            .map_err(|e| CheckpointError::Storage(e.to_string()))?;
        conn.execute_batch(CREATE_CHECKPOINTS_TABLE)
            .map_err(|e| CheckpointError::Storage(e.to_string()))?;

        Ok(Self {
            pool: Arc::new(pool),
            max_checkpoint_size,
        })
    }

    /// Set file permissions to 0600 on Unix.
    #[cfg(unix)]
    fn set_permissions_0600(path: &Path) -> Result<(), CheckpointError> {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(|e| CheckpointError::Storage(e.to_string()))
    }
}

#[allow(clippy::significant_drop_tightening)]
impl BaseCheckpointSaver for SqliteSaver {
    fn get_tuple<'a>(
        &'a self,
        config: &'a CheckpointConfig,
    ) -> synwire_core::BoxFuture<'a, Result<Option<CheckpointTuple>, CheckpointError>> {
        Box::pin(async move {
            let conn = self
                .pool
                .get()
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let (query, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) =
                config.checkpoint_id.as_ref().map_or_else(
                    || -> (&str, Vec<Box<dyn rusqlite::types::ToSql>>) {
                        (
                            "SELECT checkpoint_id, data, metadata, parent_checkpoint_id \
                             FROM checkpoints WHERE thread_id = ?1 \
                             ORDER BY rowid DESC LIMIT 1",
                            vec![Box::new(config.thread_id.clone())],
                        )
                    },
                    |checkpoint_id| {
                        (
                            "SELECT checkpoint_id, data, metadata, parent_checkpoint_id \
                             FROM checkpoints WHERE thread_id = ?1 AND checkpoint_id = ?2",
                            vec![
                                Box::new(config.thread_id.clone()),
                                Box::new(checkpoint_id.clone()),
                            ],
                        )
                    },
                );

            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(AsRef::as_ref).collect();

            let mut stmt = conn
                .prepare(query)
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let result = stmt
                .query_row(&*param_refs, |row| {
                    let checkpoint_id: String = row.get(0)?;
                    let data: Vec<u8> = row.get(1)?;
                    let metadata_json: String = row.get(2)?;
                    let parent_id: Option<String> = row.get(3)?;
                    Ok((checkpoint_id, data, metadata_json, parent_id))
                })
                .optional()
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let Some((checkpoint_id, data, metadata_json, parent_id)) = result else {
                return Ok(None);
            };

            let checkpoint: Checkpoint = serde_json::from_slice(&data)
                .map_err(|e| CheckpointError::Serialization(e.to_string()))?;
            let metadata: CheckpointMetadata = serde_json::from_str(&metadata_json)
                .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

            let tuple_config = CheckpointConfig {
                thread_id: config.thread_id.clone(),
                checkpoint_id: Some(checkpoint_id),
            };
            let parent_config = parent_id.map(|pid| CheckpointConfig {
                thread_id: config.thread_id.clone(),
                checkpoint_id: Some(pid),
            });

            Ok(Some(CheckpointTuple {
                config: tuple_config,
                checkpoint,
                metadata,
                parent_config,
            }))
        })
    }

    fn list<'a>(
        &'a self,
        config: &'a CheckpointConfig,
        limit: Option<usize>,
    ) -> synwire_core::BoxFuture<'a, Result<Vec<CheckpointTuple>, CheckpointError>> {
        Box::pin(async move {
            let conn = self
                .pool
                .get()
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let limit_val: i64 = limit
                .and_then(|l| i64::try_from(l).ok())
                .unwrap_or(i64::MAX);

            let mut stmt = conn
                .prepare(
                    "SELECT checkpoint_id, data, metadata, parent_checkpoint_id \
                     FROM checkpoints WHERE thread_id = ?1 \
                     ORDER BY rowid DESC LIMIT ?2",
                )
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let rows = stmt
                .query_map(rusqlite::params![config.thread_id, limit_val], |row| {
                    let checkpoint_id: String = row.get(0)?;
                    let data: Vec<u8> = row.get(1)?;
                    let metadata_json: String = row.get(2)?;
                    let parent_id: Option<String> = row.get(3)?;
                    Ok((checkpoint_id, data, metadata_json, parent_id))
                })
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let mut tuples = Vec::new();
            for row in rows {
                let (checkpoint_id, data, metadata_json, parent_id) =
                    row.map_err(|e| CheckpointError::Storage(e.to_string()))?;

                let checkpoint: Checkpoint = serde_json::from_slice(&data)
                    .map_err(|e| CheckpointError::Serialization(e.to_string()))?;
                let metadata: CheckpointMetadata = serde_json::from_str(&metadata_json)
                    .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

                let tuple_config = CheckpointConfig {
                    thread_id: config.thread_id.clone(),
                    checkpoint_id: Some(checkpoint_id),
                };
                let parent_config = parent_id.map(|pid| CheckpointConfig {
                    thread_id: config.thread_id.clone(),
                    checkpoint_id: Some(pid),
                });

                tuples.push(CheckpointTuple {
                    config: tuple_config,
                    checkpoint,
                    metadata,
                    parent_config,
                });
            }

            Ok(tuples)
        })
    }

    fn put<'a>(
        &'a self,
        config: &'a CheckpointConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
    ) -> synwire_core::BoxFuture<'a, Result<CheckpointConfig, CheckpointError>> {
        Box::pin(async move {
            let data = serde_json::to_vec(&checkpoint)
                .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

            if data.len() > self.max_checkpoint_size {
                return Err(CheckpointError::StateTooLarge {
                    size: data.len(),
                    max: self.max_checkpoint_size,
                });
            }

            let metadata_json = serde_json::to_string(&metadata)
                .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

            // Determine parent: the latest existing checkpoint for this thread.
            let conn = self
                .pool
                .get()
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let parent_id: Option<String> = conn
                .prepare(
                    "SELECT checkpoint_id FROM checkpoints \
                     WHERE thread_id = ?1 ORDER BY rowid DESC LIMIT 1",
                )
                .map_err(|e| CheckpointError::Storage(e.to_string()))?
                .query_row(rusqlite::params![config.thread_id], |row| row.get(0))
                .optional()
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            let _rows_changed = conn
                .execute(
                    "INSERT OR REPLACE INTO checkpoints \
                     (thread_id, checkpoint_id, data, metadata, parent_checkpoint_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        config.thread_id,
                        checkpoint.id,
                        data,
                        metadata_json,
                        parent_id,
                    ],
                )
                .map_err(|e| CheckpointError::Storage(e.to_string()))?;

            Ok(CheckpointConfig {
                thread_id: config.thread_id.clone(),
                checkpoint_id: Some(checkpoint.id),
            })
        })
    }
}

/// Extension trait for `rusqlite` optional query results.
trait OptionalExt<T> {
    /// Convert a `QueryReturnedNoRows` error to `Ok(None)`.
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use synwire_checkpoint::types::CheckpointSource;

    fn make_checkpoint(id: &str, step: i64) -> (Checkpoint, CheckpointMetadata) {
        let mut cp = Checkpoint::new(id.to_owned());
        let _prev = cp
            .channel_values
            .insert("messages".into(), serde_json::json!([]));
        let metadata = CheckpointMetadata {
            source: CheckpointSource::Loop,
            step,
            writes: HashMap::new(),
            parents: HashMap::new(),
        };
        (cp, metadata)
    }

    /// T221: `SqliteSaver` put/get/list.
    #[tokio::test]
    async fn put_get_list() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let saver = SqliteSaver::new(&db_path).unwrap();

        let config = CheckpointConfig {
            thread_id: "thread-1".into(),
            checkpoint_id: None,
        };

        // Put
        let (cp, meta) = make_checkpoint("cp-1", 0);
        let result = saver.put(&config, cp, meta).await.unwrap();
        assert_eq!(result.checkpoint_id.as_deref(), Some("cp-1"));

        // Get latest
        let tuple = saver.get_tuple(&config).await.unwrap().unwrap();
        assert_eq!(tuple.checkpoint.id, "cp-1");

        // Put second
        let (cp2, meta2) = make_checkpoint("cp-2", 1);
        let _result2 = saver.put(&config, cp2, meta2).await.unwrap();

        // Get latest should be cp-2
        let tuple = saver.get_tuple(&config).await.unwrap().unwrap();
        assert_eq!(tuple.checkpoint.id, "cp-2");
        assert!(tuple.parent_config.is_some());
        assert_eq!(
            tuple.parent_config.unwrap().checkpoint_id.as_deref(),
            Some("cp-1")
        );

        // Get specific
        let specific = CheckpointConfig {
            thread_id: "thread-1".into(),
            checkpoint_id: Some("cp-1".into()),
        };
        let tuple = saver.get_tuple(&specific).await.unwrap().unwrap();
        assert_eq!(tuple.checkpoint.id, "cp-1");

        // List
        let all = saver.list(&config, None).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].checkpoint.id, "cp-2");
        assert_eq!(all[1].checkpoint.id, "cp-1");

        // List with limit
        let limited = saver.list(&config, Some(1)).await.unwrap();
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].checkpoint.id, "cp-2");

        // Get non-existent
        let missing = CheckpointConfig {
            thread_id: "no-such-thread".into(),
            checkpoint_id: None,
        };
        assert!(saver.get_tuple(&missing).await.unwrap().is_none());
    }

    /// T222: `SqliteSaver` file permissions are 0600.
    #[cfg(unix)]
    #[tokio::test]
    async fn file_permissions_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("perms.db");
        let _saver = SqliteSaver::new(&db_path).unwrap();

        let meta = std::fs::metadata(&db_path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
    }

    /// T224: `max_checkpoint_size` enforcement.
    #[tokio::test]
    async fn max_checkpoint_size_enforcement() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("size.db");
        // Very small max size.
        let saver = SqliteSaver::with_max_size(&db_path, 10).unwrap();

        let config = CheckpointConfig {
            thread_id: "thread-1".into(),
            checkpoint_id: None,
        };

        let (cp, meta) = make_checkpoint("cp-1", 0);
        let err = saver.put(&config, cp, meta).await.unwrap_err();
        assert!(
            matches!(err, CheckpointError::StateTooLarge { .. }),
            "expected StateTooLarge, got {err:?}"
        );
    }
}

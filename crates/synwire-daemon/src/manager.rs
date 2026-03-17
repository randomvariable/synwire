//! Multi-repo/worktree manager for the Synwire daemon.
//!
//! [`RepoManager`] is the central coordinator that tracks active worktrees,
//! registers new projects by computing their [`WorktreeId`], and evicts idle
//! entries via an LRU policy when the active set exceeds the configured limit.
//!
//! # Thread safety
//!
//! All public types are `Send + Sync`.  Interior state is protected by
//! [`tokio::sync::RwLock`] so that concurrent tasks can safely register,
//! access, and evict worktrees without blocking one another for reads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use synwire_storage::{ProjectRegistry, StorageError, StorageLayout, WorktreeId};
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors produced by the [`RepoManager`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ManagerError {
    /// An error from the underlying storage layer.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    /// The requested worktree was not found in the active set.
    #[error("worktree not found: {0}")]
    NotFound(String),

    /// A worktree with this identity is already registered.
    #[error("worktree already registered: {0}")]
    AlreadyRegistered(String),
}

// ---------------------------------------------------------------------------
// WorktreeStatus
// ---------------------------------------------------------------------------

/// Runtime status of a managed worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorktreeStatus {
    /// The worktree is registered but not currently being indexed.
    Idle,
    /// An indexing pipeline is running for this worktree.
    Indexing,
    /// Indexing has completed and the worktree is ready for queries.
    Ready,
}

// ---------------------------------------------------------------------------
// WorktreeHandle
// ---------------------------------------------------------------------------

/// Per-worktree state tracked by the [`RepoManager`].
#[derive(Debug, Clone)]
pub struct WorktreeHandle {
    /// Stable identity for this worktree.
    pub worktree_id: WorktreeId,
    /// Canonical root path of the worktree on disk.
    pub root_path: PathBuf,
    /// Monotonic timestamp of the last access (used for LRU eviction).
    pub last_accessed: Instant,
    /// Current operational status.
    pub status: WorktreeStatus,
}

// ---------------------------------------------------------------------------
// RepoManager
// ---------------------------------------------------------------------------

/// Manages the set of active repositories and worktrees for the daemon.
///
/// The manager keeps at most `max_active` worktrees in the active set.  When
/// this limit is exceeded, [`evict_idle`](Self::evict_idle) removes the
/// least-recently-used entries.
pub struct RepoManager {
    /// Storage layout for computing paths and persisting the registry.
    layout: StorageLayout,
    /// Persistent global project registry (shared across daemon restarts).
    registry: Arc<RwLock<ProjectRegistry>>,
    /// Currently active worktrees, keyed by [`WorktreeId::key`].
    active_worktrees: Arc<RwLock<HashMap<String, WorktreeHandle>>>,
    /// Maximum number of worktrees to keep active simultaneously.
    max_active: usize,
}

impl RepoManager {
    /// Create a new `RepoManager`.
    ///
    /// `max_active` controls the upper bound on concurrently tracked worktrees.
    /// The persistent [`ProjectRegistry`] is loaded from the storage layout; if
    /// the registry file does not yet exist an empty registry is used.
    pub fn new(layout: StorageLayout, max_active: usize) -> Result<Self, ManagerError> {
        let registry = ProjectRegistry::load(&layout)?;
        Ok(Self {
            layout,
            registry: Arc::new(RwLock::new(registry)),
            active_worktrees: Arc::new(RwLock::new(HashMap::new())),
            max_active,
        })
    }

    /// Register a worktree by its root path.
    ///
    /// Computes the [`WorktreeId`] from the path, adds the worktree to the
    /// persistent registry, and inserts it into the active set with
    /// [`WorktreeStatus::Idle`].
    ///
    /// Returns the computed `WorktreeId` on success.
    ///
    /// # Errors
    ///
    /// - [`ManagerError::AlreadyRegistered`] if the worktree is already active.
    /// - [`ManagerError::Storage`] if the `WorktreeId` cannot be computed or the
    ///   registry cannot be persisted.
    pub async fn register(&self, root_path: &Path) -> Result<WorktreeId, ManagerError> {
        let wid = WorktreeId::for_path(root_path)?;
        let key = wid.key();

        // Check the active set first (read lock).
        {
            let active = self.active_worktrees.read().await;
            if active.contains_key(&key) {
                return Err(ManagerError::AlreadyRegistered(key));
            }
        }

        // Persist to the global registry.
        {
            let mut reg = self.registry.write().await;
            reg.upsert(&wid, root_path);
            if let Err(e) = reg.save(&self.layout) {
                warn!(key = %key, "failed to persist registry after upsert: {e}");
                // Non-fatal: the in-memory state is still consistent.
            }
        }

        // Insert into the active set.
        let canonical = root_path.canonicalize().map_err(StorageError::from)?;
        let handle = WorktreeHandle {
            worktree_id: wid.clone(),
            root_path: canonical,
            last_accessed: Instant::now(),
            status: WorktreeStatus::Idle,
        };

        {
            let mut active = self.active_worktrees.write().await;
            let _ = active.insert(key.clone(), handle);
        }

        info!(key = %key, "worktree registered");
        Ok(wid)
    }

    /// Retrieve a clone of the [`WorktreeHandle`] for the given identity.
    ///
    /// Returns `None` if the worktree is not in the active set.
    pub async fn get(&self, worktree_id: &WorktreeId) -> Option<WorktreeHandle> {
        let active = self.active_worktrees.read().await;
        active.get(&worktree_id.key()).cloned()
    }

    /// Update the `last_accessed` timestamp for an active worktree, keeping it
    /// alive during LRU eviction.
    pub async fn touch(&self, worktree_id: &WorktreeId) {
        {
            let mut active = self.active_worktrees.write().await;
            if let Some(handle) = active.get_mut(&worktree_id.key()) {
                handle.last_accessed = Instant::now();
                debug!(key = %worktree_id.key(), "worktree touched");
            }
        }

        // Also update the persistent registry timestamp.
        let mut reg = self.registry.write().await;
        reg.touch(worktree_id);
        // Best-effort persist — failure is non-fatal.
        if let Err(e) = reg.save(&self.layout) {
            warn!(key = %worktree_id.key(), "failed to persist registry after touch: {e}");
        }
        drop(reg);
    }

    /// List all active worktree handles.
    ///
    /// The returned vector is in no particular order.
    pub async fn list_active(&self) -> Vec<WorktreeHandle> {
        let active = self.active_worktrees.read().await;
        active.values().cloned().collect()
    }

    /// Evict the least-recently-used worktrees when the active set exceeds
    /// `max_active`.
    ///
    /// Only worktrees with [`WorktreeStatus::Idle`] or [`WorktreeStatus::Ready`]
    /// are eligible for eviction; actively indexing worktrees are skipped.
    ///
    /// Returns the list of evicted [`WorktreeId`]s.
    pub async fn evict_idle(&self) -> Vec<WorktreeId> {
        let mut active = self.active_worktrees.write().await;

        if active.len() <= self.max_active {
            return Vec::new();
        }

        let to_evict = active.len() - self.max_active;

        // Collect eligible entries and sort by last_accessed ascending (oldest first).
        let mut candidates: Vec<(String, Instant)> = active
            .iter()
            .filter(|(_, h)| h.status != WorktreeStatus::Indexing)
            .map(|(k, h)| (k.clone(), h.last_accessed))
            .collect();
        candidates.sort_by_key(|(_k, t)| *t);

        let mut evicted = Vec::with_capacity(to_evict);
        for (key, _) in candidates.into_iter().take(to_evict) {
            if let Some(handle) = active.remove(&key) {
                info!(key = %key, "evicting idle worktree");
                evicted.push(handle.worktree_id);
            }
        }

        evicted
    }

    /// Remove a worktree from the active set and the persistent registry.
    ///
    /// Returns `true` if the worktree was present and removed, `false`
    /// otherwise.
    pub async fn unregister(&self, worktree_id: &WorktreeId) -> bool {
        let key = worktree_id.key();
        let removed = {
            let mut active = self.active_worktrees.write().await;
            active.remove(&key).is_some()
        };

        if removed {
            let mut reg = self.registry.write().await;
            reg.remove(worktree_id);
            if let Err(e) = reg.save(&self.layout) {
                warn!(key = %key, "failed to persist registry after unregister: {e}");
            }
            drop(reg);
            info!(key = %key, "worktree unregistered");
        }

        removed
    }

    /// Update the [`WorktreeStatus`] for an active worktree.
    ///
    /// # Errors
    ///
    /// Returns [`ManagerError::NotFound`] if the worktree is not in the active
    /// set.
    pub async fn set_status(
        &self,
        worktree_id: &WorktreeId,
        status: WorktreeStatus,
    ) -> Result<(), ManagerError> {
        let key = worktree_id.key();
        self.active_worktrees
            .write()
            .await
            .get_mut(&key)
            .ok_or_else(|| ManagerError::NotFound(key.clone()))?
            .status = status;
        debug!(key = %key, ?status, "worktree status updated");
        Ok(())
    }

    /// Return the number of currently active worktrees.
    pub async fn active_count(&self) -> usize {
        self.active_worktrees.read().await.len()
    }

    /// Return the configured maximum number of active worktrees.
    #[must_use]
    pub const fn max_active(&self) -> usize {
        self.max_active
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use synwire_storage::identity::RepoId;
    use tempfile::tempdir;

    fn test_layout() -> (StorageLayout, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let layout = StorageLayout::with_root(dir.path(), "synwire");
        (layout, dir)
    }

    fn dummy_worktree(name: &str) -> WorktreeId {
        WorktreeId::from_parts(
            RepoId::from_string(format!("repo-{name}")),
            format!("{name}hash000000"),
            format!("{name}@main"),
        )
    }

    #[tokio::test]
    async fn new_manager_starts_empty() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");
        assert_eq!(mgr.active_count().await, 0);
        assert_eq!(mgr.max_active(), 10);
    }

    #[tokio::test]
    async fn register_and_get_with_real_path() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");

        // Use a real temporary directory so canonicalize works.
        let worktree_dir = tempdir().expect("worktree_dir");
        let wid = mgr.register(worktree_dir.path()).await.expect("register");

        let handle = mgr.get(&wid).await.expect("get returned None");
        assert_eq!(handle.worktree_id, wid);
        assert_eq!(handle.status, WorktreeStatus::Idle);
    }

    #[tokio::test]
    async fn double_register_is_error() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");

        let worktree_dir = tempdir().expect("worktree_dir");
        let _wid = mgr.register(worktree_dir.path()).await.expect("register");
        let err = mgr.register(worktree_dir.path()).await.unwrap_err();
        assert!(matches!(err, ManagerError::AlreadyRegistered(_)));
    }

    #[tokio::test]
    async fn unregister_removes_worktree() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");

        let worktree_dir = tempdir().expect("worktree_dir");
        let wid = mgr.register(worktree_dir.path()).await.expect("register");
        assert!(mgr.unregister(&wid).await);
        assert!(mgr.get(&wid).await.is_none());
        // Unregistering again returns false.
        assert!(!mgr.unregister(&wid).await);
    }

    #[tokio::test]
    async fn evict_idle_respects_max_active() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 2).expect("new");

        // Insert three worktrees directly into the active set.
        let ids: Vec<WorktreeId> = (0..3).map(|i| dummy_worktree(&format!("w{i}"))).collect();
        {
            let mut active = mgr.active_worktrees.write().await;
            for (i, wid) in ids.iter().enumerate() {
                let _ = active.insert(
                    wid.key(),
                    WorktreeHandle {
                        worktree_id: wid.clone(),
                        root_path: PathBuf::from(format!("/tmp/w{i}")),
                        last_accessed: Instant::now()
                            - std::time::Duration::from_secs((3 - i as u64) * 10),
                        status: WorktreeStatus::Idle,
                    },
                );
            }
            drop(active);
        }

        assert_eq!(mgr.active_count().await, 3);
        let evicted = mgr.evict_idle().await;
        assert_eq!(evicted.len(), 1);
        assert_eq!(mgr.active_count().await, 2);
        // The oldest entry should have been evicted.
        assert_eq!(evicted[0].key(), ids[0].key());
    }

    #[tokio::test]
    async fn evict_skips_indexing_worktrees() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 1).expect("new");

        let idle_wid = dummy_worktree("idle");
        let indexing_wid = dummy_worktree("indexing");

        {
            let mut active = mgr.active_worktrees.write().await;
            let _ = active.insert(
                idle_wid.key(),
                WorktreeHandle {
                    worktree_id: idle_wid.clone(),
                    root_path: PathBuf::from("/tmp/idle"),
                    // The idle entry is newer (more recently accessed).
                    last_accessed: Instant::now(),
                    status: WorktreeStatus::Idle,
                },
            );
            let _ = active.insert(
                indexing_wid.key(),
                WorktreeHandle {
                    worktree_id: indexing_wid.clone(),
                    root_path: PathBuf::from("/tmp/indexing"),
                    // The indexing entry is oldest but should be skipped.
                    last_accessed: Instant::now() - std::time::Duration::from_secs(100),
                    status: WorktreeStatus::Indexing,
                },
            );
        }

        let evicted = mgr.evict_idle().await;
        // Only the idle entry can be evicted.
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].key(), idle_wid.key());
    }

    #[tokio::test]
    async fn list_active_returns_all() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");

        let wid_a = dummy_worktree("a");
        let wid_b = dummy_worktree("b");
        {
            let mut active = mgr.active_worktrees.write().await;
            for wid in [&wid_a, &wid_b] {
                let _ = active.insert(
                    wid.key(),
                    WorktreeHandle {
                        worktree_id: wid.clone(),
                        root_path: PathBuf::from("/tmp"),
                        last_accessed: Instant::now(),
                        status: WorktreeStatus::Ready,
                    },
                );
            }
        }

        let listed = mgr.list_active().await;
        assert_eq!(listed.len(), 2);
    }

    #[tokio::test]
    async fn set_status_updates_handle() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");

        let wid = dummy_worktree("s");
        {
            let mut active = mgr.active_worktrees.write().await;
            let _ = active.insert(
                wid.key(),
                WorktreeHandle {
                    worktree_id: wid.clone(),
                    root_path: PathBuf::from("/tmp"),
                    last_accessed: Instant::now(),
                    status: WorktreeStatus::Idle,
                },
            );
        }

        mgr.set_status(&wid, WorktreeStatus::Indexing)
            .await
            .expect("set_status");
        let handle = mgr.get(&wid).await.expect("get");
        assert_eq!(handle.status, WorktreeStatus::Indexing);
    }

    #[tokio::test]
    async fn set_status_not_found() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");

        let wid = dummy_worktree("missing");
        let err = mgr
            .set_status(&wid, WorktreeStatus::Ready)
            .await
            .unwrap_err();
        assert!(matches!(err, ManagerError::NotFound(_)));
    }

    #[tokio::test]
    async fn no_eviction_when_under_limit() {
        let (layout, _dir) = test_layout();
        let mgr = RepoManager::new(layout, 10).expect("new");

        let wid = dummy_worktree("only");
        {
            let mut active = mgr.active_worktrees.write().await;
            let _ = active.insert(
                wid.key(),
                WorktreeHandle {
                    worktree_id: wid.clone(),
                    root_path: PathBuf::from("/tmp"),
                    last_accessed: Instant::now(),
                    status: WorktreeStatus::Idle,
                },
            );
        }

        let evicted = mgr.evict_idle().await;
        assert!(evicted.is_empty());
        assert_eq!(mgr.active_count().await, 1);
    }
}

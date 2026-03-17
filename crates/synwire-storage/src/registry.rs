//! Global project registry.
//!
//! The registry tracks all Synwire-indexed projects across a product
//! installation, persisted as `global/registry.json`.  Each entry records the
//! worktree identity, last-access timestamp, and optional user-supplied tags.

use crate::{StorageError, StorageLayout, WorktreeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A single entry in the project registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RegistryEntry {
    /// Stable worktree identifier.
    pub worktree_id: WorktreeId,
    /// Canonical root path of the worktree at the time of registration.
    pub root_path: String,
    /// RFC 3339 timestamp of the last access.
    pub last_accessed_at: String,
    /// User-supplied tags for filtering.
    pub tags: Vec<String>,
}

/// In-memory representation of the global project registry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectRegistry {
    /// Entries keyed by the [`WorktreeId::key`].
    pub entries: HashMap<String, RegistryEntry>,
}

impl ProjectRegistry {
    /// Load the registry from `StorageLayout::global_registry()`.
    ///
    /// Returns an empty registry if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the file exists but cannot be parsed.
    pub fn load(layout: &StorageLayout) -> Result<Self, StorageError> {
        let path = layout.global_registry();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        let reg: Self = serde_json::from_str(&data)?;
        Ok(reg)
    }

    /// Persist the registry to `StorageLayout::global_registry()`.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the parent directory cannot be created or
    /// the file cannot be written.
    pub fn save(&self, layout: &StorageLayout) -> Result<(), StorageError> {
        let path = layout.global_registry();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Register or update an entry for the given worktree.
    pub fn upsert(&mut self, worktree: &WorktreeId, root_path: &Path) {
        let key = worktree.key();
        let _ = self.entries.insert(
            key,
            RegistryEntry {
                worktree_id: worktree.clone(),
                root_path: root_path.display().to_string(),
                last_accessed_at: chrono::Utc::now().to_rfc3339(),
                tags: Vec::new(),
            },
        );
    }

    /// Update the last-access timestamp for a worktree (if registered).
    pub fn touch(&mut self, worktree: &WorktreeId) {
        if let Some(entry) = self.entries.get_mut(&worktree.key()) {
            entry.last_accessed_at = chrono::Utc::now().to_rfc3339();
        }
    }

    /// Remove a worktree from the registry.
    pub fn remove(&mut self, worktree: &WorktreeId) {
        let _ = self.entries.remove(&worktree.key());
    }

    /// Add a tag to an existing entry.
    pub fn add_tag(&mut self, worktree: &WorktreeId, tag: impl Into<String>) {
        let tag = tag.into();
        if let Some(entry) = self.entries.get_mut(&worktree.key()) {
            if !entry.tags.contains(&tag) {
                entry.tags.push(tag);
            }
        }
    }

    /// Look up an entry by worktree key.
    #[must_use]
    pub fn get(&self, worktree: &WorktreeId) -> Option<&RegistryEntry> {
        self.entries.get(&worktree.key())
    }

    /// Return all entries, sorted by `last_accessed_at` descending.
    #[must_use]
    pub fn list_recent(&self) -> Vec<&RegistryEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by(|a, b| b.last_accessed_at.cmp(&a.last_accessed_at));
        entries
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::identity::RepoId;
    use tempfile::tempdir;

    fn dummy_worktree(name: &str) -> WorktreeId {
        WorktreeId::from_parts(
            RepoId::from_string(format!("repo-{name}")),
            format!("{name}hash000000"),
            format!("{name}@main"),
        )
    }

    fn test_layout() -> (StorageLayout, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let layout = StorageLayout::with_root(dir.path(), "synwire");
        (layout, dir)
    }

    #[test]
    fn empty_registry_when_absent() {
        let (layout, _dir) = test_layout();
        let reg = ProjectRegistry::load(&layout).expect("load");
        assert!(reg.entries.is_empty());
    }

    #[test]
    fn upsert_and_round_trip() {
        let (layout, _dir) = test_layout();
        let wid = dummy_worktree("a");
        let root = std::path::PathBuf::from("/tmp/my-repo");
        let mut reg = ProjectRegistry::load(&layout).expect("load");
        reg.upsert(&wid, &root);
        reg.save(&layout).expect("save");

        let reg2 = ProjectRegistry::load(&layout).expect("reload");
        assert!(reg2.get(&wid).is_some());
        assert_eq!(reg2.get(&wid).expect("entry").root_path, "/tmp/my-repo");
    }

    #[test]
    fn list_recent_orders_by_timestamp() {
        let (_layout, _dir) = test_layout();
        let wa = dummy_worktree("a");
        let wb = dummy_worktree("b");
        let root = std::path::PathBuf::from("/tmp");
        let mut reg = ProjectRegistry::default();
        reg.upsert(&wa, &root);
        // Sleep briefly so timestamps differ.
        std::thread::sleep(std::time::Duration::from_millis(5));
        reg.upsert(&wb, &root);
        let recent = reg.list_recent();
        assert_eq!(recent[0].worktree_id.key(), wb.key());
    }

    #[test]
    fn tag_added_and_present() {
        let wid = dummy_worktree("tagged");
        let root = std::path::PathBuf::from("/tmp");
        let mut reg = ProjectRegistry::default();
        reg.upsert(&wid, &root);
        reg.add_tag(&wid, "important");
        assert!(
            reg.get(&wid)
                .expect("entry")
                .tags
                .contains(&"important".to_owned())
        );
    }

    #[test]
    fn remove_entry() {
        let wid = dummy_worktree("rm");
        let root = std::path::PathBuf::from("/tmp");
        let mut reg = ProjectRegistry::default();
        reg.upsert(&wid, &root);
        reg.remove(&wid);
        assert!(reg.get(&wid).is_none());
    }
}

//! Two-level project identity: [`RepoId`] (repository family) and [`WorktreeId`]
//! (specific working copy).
//!
//! `RepoId` is the SHA-1 of the Git repository's first commit, so it is stable
//! across clones and worktrees of the same repository.  When Git is unavailable
//! the SHA-256 of the canonical directory path is used as a fallback.
//!
//! `WorktreeId` uniquely identifies a specific working copy within a repository
//! family.  It combines the `RepoId` with a SHA-256 of the canonicalised
//! worktree root path.

use crate::StorageError;
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Stable identifier for a repository *family* — shared across all worktrees
/// and clones of the same repository.
///
/// Derived from the SHA-1 of the first (root) commit, or a SHA-256 hash of the
/// canonical directory path when Git is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RepoId(String);

impl RepoId {
    /// Compute the `RepoId` for the repository that contains `path`.
    ///
    /// Runs `git rev-list --max-parents=0 HEAD` in the directory.  Falls back
    /// to `sha256(canonical_path)` if Git is not installed or the directory is
    /// not a Git repository.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Io`] if the path cannot be canonicalised.
    pub fn for_path(path: &Path) -> Result<Self, StorageError> {
        let canonical = path.canonicalize()?;

        // Try git first-commit hash.
        if let Some(hash) = git_first_commit(&canonical) {
            return Ok(Self(hash));
        }

        // Fallback: SHA-256 of the canonical path string.
        Ok(Self(sha256_hex(canonical.to_string_lossy().as_bytes())))
    }

    /// Create a `RepoId` from a pre-computed string value.
    ///
    /// Useful for deserialising stored identifiers.
    #[must_use]
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the underlying string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identifier for a specific *working copy* (worktree) within a
/// repository family.
///
/// Combines the [`RepoId`] with a SHA-256 of the canonicalised worktree root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WorktreeId {
    /// Repository-level identity (shared across worktrees).
    pub repo_id: RepoId,
    /// Per-worktree discriminator derived from the canonicalised root path.
    pub worktree_hash: String,
    /// Human-readable display name (`<repo_name>@<branch>`).
    pub display_name: String,
}

impl WorktreeId {
    /// Compute the `WorktreeId` for the worktree that contains `path`.
    ///
    /// Runs `git rev-parse --show-toplevel` to find the worktree root, then
    /// computes the repo identity and the per-worktree hash.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Io`] if the path cannot be canonicalised.
    pub fn for_path(path: &Path) -> Result<Self, StorageError> {
        let canonical = path.canonicalize()?;
        let worktree_root = git_worktree_root(&canonical).unwrap_or_else(|| canonical.clone());
        let repo_id = RepoId::for_path(&worktree_root)?;
        let worktree_hash = sha256_hex(worktree_root.to_string_lossy().as_bytes());
        let display_name = build_display_name(&worktree_root);
        Ok(Self {
            repo_id,
            worktree_hash,
            display_name,
        })
    }

    /// Create a `WorktreeId` from pre-computed components.
    #[must_use]
    pub const fn from_parts(repo_id: RepoId, worktree_hash: String, display_name: String) -> Self {
        Self {
            repo_id,
            worktree_hash,
            display_name,
        }
    }

    /// Return a compact string key suitable for use in directory names.
    #[must_use]
    pub fn key(&self) -> String {
        let prefix_len = self.worktree_hash.len().min(12);
        format!("{}-{}", self.repo_id, &self.worktree_hash[..prefix_len])
    }
}

impl fmt::Display for WorktreeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn git_first_commit(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-list", "--max-parents=0", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let hash = s.trim().to_owned();
    if hash.is_empty() { None } else { Some(hash) }
}

fn git_worktree_root(dir: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(s.trim());
    if path.exists() { Some(path) } else { None }
}

fn git_branch(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let branch = s.trim().to_owned();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn build_display_name(worktree_root: &Path) -> String {
    let repo_name = worktree_root
        .file_name()
        .map_or("unknown", |n| n.to_str().unwrap_or("unknown"));
    let branch = git_branch(worktree_root).unwrap_or_else(|| "main".to_owned());
    format!("{repo_name}@{branch}")
}

fn sha256_hex(input: &[u8]) -> String {
    let hash = Sha256::digest(input);
    format!("{hash:x}")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn repo_id_fallback_is_deterministic() {
        let dir = env::temp_dir();
        let id1 = RepoId::for_path(&dir).expect("RepoId::for_path failed");
        let id2 = RepoId::for_path(&dir).expect("RepoId::for_path failed");
        assert_eq!(id1, id2);
    }

    #[test]
    fn worktree_id_key_is_short() {
        let dir = env::temp_dir();
        let wid = WorktreeId::for_path(&dir).expect("WorktreeId::for_path failed");
        // key should be non-empty and reasonably compact
        assert!(!wid.key().is_empty());
        assert!(wid.key().len() < 80);
    }

    #[test]
    fn repo_id_display() {
        let id = RepoId::from_string("abc123");
        assert_eq!(id.to_string(), "abc123");
    }
}

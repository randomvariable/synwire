//! Cache directory management for index metadata.
//!
//! Uses [`StorageLayout`] to determine the index cache directory when
//! available, falling back to the platform's default cache directory
//! (XDG on Linux, `~/Library/Caches` on macOS, `%LOCALAPPDATA%` on Windows)
//! via the `directories` crate for backward compatibility.

use crate::config::IndexConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use synwire_storage::{StorageLayout, WorktreeId};

/// Persistent metadata for a completed index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMeta {
    /// Canonical path that was indexed.
    pub path: String,
    /// RFC 3339 timestamp of the last index run.
    pub indexed_at: String,
    /// Number of files indexed.
    pub files_indexed: usize,
    /// Number of chunks produced.
    pub chunks_produced: usize,
    /// Schema version for forward-compatibility.
    pub version: u32,
}

/// Return the cache directory using a [`StorageLayout`] and [`WorktreeId`].
///
/// Layout: `$CACHE_DIR/<product>/indices/<worktree_key>/`
///
/// Prefer this over [`cache_dir`] when `StorageLayout` is available.
/// Used by `synwire-mcp-server` for the `index_status` tool.
#[must_use]
#[allow(dead_code)]
pub fn cache_dir_from_layout(layout: &StorageLayout, worktree: &WorktreeId) -> PathBuf {
    layout.index_cache(worktree)
}

/// Return the cache directory for a given canonical path (legacy API).
///
/// Layout: `$CACHE_DIR/synwire/indices/<sha256(path)>/`
///
/// For new code, prefer [`cache_dir_from_layout`] which respects the product
/// name and `StorageLayout` config hierarchy.
pub fn cache_dir(config: &IndexConfig, canonical: &Path) -> PathBuf {
    let hash = Sha256::digest(canonical.to_string_lossy().as_bytes());
    let hex = format!("{hash:x}");
    let base = config.cache_base.clone().unwrap_or_else(default_cache_base);
    base.join("synwire").join("indices").join(hex)
}

/// Read metadata from `<cache_dir>/meta.json`.  Returns `None` if absent or corrupt.
pub fn read_meta(cache: &Path) -> Option<IndexMeta> {
    let path = cache.join("meta.json");
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Write metadata to `<cache_dir>/meta.json`.
///
/// # Errors
///
/// Returns an I/O error if the directory cannot be created or the file written.
pub fn write_meta(cache: &Path, meta: &IndexMeta) -> std::io::Result<()> {
    std::fs::create_dir_all(cache)?;
    let json = serde_json::to_string_pretty(meta).map_err(std::io::Error::other)?;
    std::fs::write(cache.join("meta.json"), json)
}

fn default_cache_base() -> PathBuf {
    directories::BaseDirs::new()
        .map_or_else(|| PathBuf::from(".cache"), |d| d.cache_dir().to_path_buf())
}

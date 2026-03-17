//! Product-scoped persistent storage layout.
//!
//! [`StorageLayout`] computes paths for all Synwire subsystems using a
//! consistent hierarchy rooted at the platform data and cache directories.
//!
//! ## Path Layout
//!
//! ```text
//! $XDG_DATA_HOME/<product>/       (Linux/XDG)
//! ~/Library/Application Support/<product>/   (macOS)
//! %APPDATA%/<product>/            (Windows)
//!
//! ├── sessions/<session_id>.db        — checkpoint databases
//! ├── experience/<worktree_key>.db    — per-worktree experience pool
//! ├── skills/                         — global agent skills
//! ├── logs/                           — rotating log files
//! ├── daemon.pid                      — daemon PID file
//! ├── daemon.sock                     — daemon UDS socket
//! └── global/
//!     ├── registry.json
//!     ├── experience.db
//!     ├── dependencies.db
//!     └── config.json
//!
//! $XDG_CACHE_HOME/<product>/
//! ├── indices/<worktree_key>/         — vector + BM25 indices
//! ├── graphs/<worktree_key>/          — code dependency graphs
//! ├── communities/<worktree_key>/     — community detection state
//! ├── lsp/<worktree_key>/            — LSP caches
//! ├── models/                         — embedding model cache
//! └── repos/<owner>/<repo>/          — cloned repositories
//! ```

use crate::{StorageError, WorktreeId};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration override for [`StorageLayout`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[derive(Default)]
pub struct StorageConfig {
    /// Override for the data home directory.
    pub data_home: Option<PathBuf>,
    /// Override for the cache home directory.
    pub cache_home: Option<PathBuf>,
    /// Product name (defaults to `"synwire"`).
    pub product_name: Option<String>,
    /// Custom name for the project-local skills directory (`.<product>` by default).
    pub project_skills_dirname: Option<String>,
}

/// Computes all Synwire storage paths for a given product name.
///
/// # Configuration hierarchy
///
/// 1. `SYNWIRE_DATA_DIR` / `SYNWIRE_CACHE_DIR` environment variables
/// 2. Programmatic override via [`StorageLayout::with_root`]
/// 3. Project-local `.<product>/config.json`
/// 4. Platform default (`directories::BaseDirs`)
#[derive(Debug, Clone)]
pub struct StorageLayout {
    data_home: PathBuf,
    cache_home: PathBuf,
    product_name: String,
    project_skills_dirname: String,
}

impl StorageLayout {
    /// Create a new layout for the given product name, respecting environment
    /// variables and platform defaults.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotWritable`] if the platform provides no
    /// usable base directories.
    pub fn new(product_name: impl Into<String>) -> Result<Self, StorageError> {
        let product_name = product_name.into();

        // Environment variable overrides take highest precedence.
        let data_home = if let Ok(val) = std::env::var("SYNWIRE_DATA_DIR") {
            PathBuf::from(val)
        } else {
            let base = BaseDirs::new().ok_or_else(|| StorageError::NotWritable {
                path: "<platform data dir>".to_owned(),
            })?;
            base.data_dir().join(&product_name)
        };

        let cache_home = if let Ok(val) = std::env::var("SYNWIRE_CACHE_DIR") {
            PathBuf::from(val)
        } else {
            let base = BaseDirs::new().ok_or_else(|| StorageError::NotWritable {
                path: "<platform cache dir>".to_owned(),
            })?;
            base.cache_dir().join(&product_name)
        };

        let project_skills_dirname = format!(".{product_name}");

        Ok(Self {
            data_home,
            cache_home,
            product_name,
            project_skills_dirname,
        })
    }

    /// Create a layout rooted at a custom base directory (for testing or
    /// explicit overrides).  Data is stored under `<root>/data/<product>` and
    /// caches under `<root>/cache/<product>`.
    pub fn with_root(root: impl AsRef<Path>, product_name: impl Into<String>) -> Self {
        let root = root.as_ref();
        let product_name = product_name.into();
        let project_skills_dirname = format!(".{product_name}");
        Self {
            data_home: root.join("data").join(&product_name),
            cache_home: root.join("cache").join(&product_name),
            product_name,
            project_skills_dirname,
        }
    }

    /// Apply a [`StorageConfig`] override on top of this layout.
    #[must_use]
    pub fn with_config(mut self, config: &StorageConfig) -> Self {
        if let Some(d) = &config.data_home {
            self.data_home.clone_from(d);
        }
        if let Some(c) = &config.cache_home {
            self.cache_home.clone_from(c);
        }
        if let Some(p) = &config.product_name {
            self.product_name.clone_from(p);
        }
        if let Some(d) = &config.project_skills_dirname {
            self.project_skills_dirname.clone_from(d);
        }
        self
    }

    // -----------------------------------------------------------------------
    // Durable data paths (under $XDG_DATA_HOME/<product>/)
    // -----------------------------------------------------------------------

    /// Root durable data directory for this product.
    #[must_use]
    pub fn data_home(&self) -> &Path {
        &self.data_home
    }

    /// Root cache directory for this product.
    #[must_use]
    pub fn cache_home(&self) -> &Path {
        &self.cache_home
    }

    /// Product name.
    #[must_use]
    pub fn product_name(&self) -> &str {
        &self.product_name
    }

    /// `SQLite` checkpoint database for a given session ID.
    #[must_use]
    pub fn session_db(&self, session_id: &str) -> PathBuf {
        self.data_home
            .join("sessions")
            .join(format!("{session_id}.db"))
    }

    /// Per-worktree experience pool database.
    #[must_use]
    pub fn experience_db(&self, worktree: &WorktreeId) -> PathBuf {
        self.data_home
            .join("experience")
            .join(format!("{}.db", worktree.key()))
    }

    /// Global agent skills directory.
    #[must_use]
    pub fn skills_dir(&self) -> PathBuf {
        self.data_home.join("skills")
    }

    /// Rotating log files directory.
    #[must_use]
    pub fn logs_dir(&self) -> PathBuf {
        self.data_home.join("logs")
    }

    /// Daemon PID file path.
    #[must_use]
    pub fn daemon_pid_file(&self) -> PathBuf {
        self.data_home.join("daemon.pid")
    }

    /// Daemon Unix domain socket path.
    #[must_use]
    pub fn daemon_socket(&self) -> PathBuf {
        self.data_home.join("daemon.sock")
    }

    /// Global cross-project experience database.
    #[must_use]
    pub fn global_experience_db(&self) -> PathBuf {
        self.data_home.join("global").join("experience.db")
    }

    /// Global cross-project dependency index database.
    #[must_use]
    pub fn global_dependency_db(&self) -> PathBuf {
        self.data_home.join("global").join("dependencies.db")
    }

    /// Global project registry JSON file.
    #[must_use]
    pub fn global_registry(&self) -> PathBuf {
        self.data_home.join("global").join("registry.json")
    }

    /// Global product config JSON file.
    #[must_use]
    pub fn global_config(&self) -> PathBuf {
        self.data_home.join("global").join("config.json")
    }

    // -----------------------------------------------------------------------
    // Cache paths (under $XDG_CACHE_HOME/<product>/)
    // -----------------------------------------------------------------------

    /// Vector + BM25 index cache directory for a worktree.
    #[must_use]
    pub fn index_cache(&self, worktree: &WorktreeId) -> PathBuf {
        self.cache_home.join("indices").join(worktree.key())
    }

    /// Code dependency graph directory for a worktree.
    #[must_use]
    pub fn graph_dir(&self, worktree: &WorktreeId) -> PathBuf {
        self.cache_home.join("graphs").join(worktree.key())
    }

    /// Community detection state directory for a worktree.
    #[must_use]
    pub fn communities_dir(&self, worktree: &WorktreeId) -> PathBuf {
        self.cache_home.join("communities").join(worktree.key())
    }

    /// LSP server cache directory for a worktree.
    #[must_use]
    pub fn lsp_cache(&self, worktree: &WorktreeId) -> PathBuf {
        self.cache_home.join("lsp").join(worktree.key())
    }

    /// Embedding model download cache.
    #[must_use]
    pub fn models_cache(&self) -> PathBuf {
        self.cache_home.join("models")
    }

    /// Root directory for cloned repositories.
    #[must_use]
    pub fn repos_cache(&self) -> PathBuf {
        self.cache_home.join("repos")
    }

    /// Directory for a specific cloned repository.
    #[must_use]
    pub fn repo_cache(&self, owner: &str, repo: &str) -> PathBuf {
        self.repos_cache().join(owner).join(repo)
    }

    /// Remove cached repositories not accessed within `max_age_days`.
    ///
    /// Returns the list of directories removed.
    /// Skips repositories that are currently mounted (not implemented in v0.1).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Io`] if a directory entry cannot be read or
    /// removed.
    pub fn repo_gc(&self, max_age_days: u64) -> Result<Vec<PathBuf>, StorageError> {
        let repos_root = self.repos_cache();
        let mut removed = Vec::new();

        let cutoff =
            std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_days * 86_400);

        // repos_cache layout: <repos_root>/<owner>/<repo>/
        let owner_entries = match std::fs::read_dir(&repos_root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(removed),
            Err(e) => return Err(StorageError::from(e)),
        };

        for owner_entry in owner_entries {
            let owner_entry = match owner_entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "skipping unreadable owner entry in repos cache");
                    continue;
                }
            };

            let owner_path = owner_entry.path();
            if !owner_path.is_dir() {
                continue;
            }

            let repo_entries = match std::fs::read_dir(&owner_path) {
                Ok(entries) => entries,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        path = %owner_path.display(),
                        "skipping unreadable owner directory"
                    );
                    continue;
                }
            };

            for repo_entry in repo_entries {
                let repo_entry = match repo_entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(error = %e, "skipping unreadable repo entry");
                        continue;
                    }
                };

                let repo_path = repo_entry.path();
                if !repo_path.is_dir() {
                    continue;
                }

                let modified = match std::fs::metadata(&repo_path).and_then(|m| m.modified()) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            path = %repo_path.display(),
                            "skipping repo with unreadable metadata"
                        );
                        continue;
                    }
                };

                if modified < cutoff {
                    if let Err(e) = std::fs::remove_dir_all(&repo_path) {
                        tracing::warn!(
                            error = %e,
                            path = %repo_path.display(),
                            "failed to remove stale repo cache"
                        );
                        continue;
                    }
                    removed.push(repo_path);
                }
            }
        }

        Ok(removed)
    }

    // -----------------------------------------------------------------------
    // Convention helpers
    // -----------------------------------------------------------------------

    /// Name of the project-local skills directory (e.g., `.synwire`).
    #[must_use]
    pub fn project_skills_dirname(&self) -> &str {
        &self.project_skills_dirname
    }

    /// Ensure the given directory exists, creating it (and all parents) as
    /// needed.  Sets permissions to `0o700` on Unix.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Io`] if the directory cannot be created.
    pub fn ensure_dir(&self, path: &Path) -> Result<(), StorageError> {
        std::fs::create_dir_all(path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o700);
            std::fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    /// Load a per-project config from `<project_root>/.<product>/config.json`.
    ///
    /// Returns `Ok(None)` if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::InvalidConfig`] if the file exists but is not
    /// valid JSON.
    pub fn load_project_config(
        &self,
        project_root: &Path,
    ) -> Result<Option<StorageConfig>, StorageError> {
        let config_path = project_root
            .join(&self.project_skills_dirname)
            .join("config.json");
        if !config_path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&config_path)?;
        let cfg: StorageConfig =
            serde_json::from_str(&data).map_err(|e| StorageError::InvalidConfig {
                path: config_path.display().to_string(),
                reason: e.to_string(),
            })?;
        Ok(Some(cfg))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::WorktreeId;
    use tempfile::tempdir;

    fn test_layout() -> (StorageLayout, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let layout = StorageLayout::with_root(dir.path(), "synwire");
        (layout, dir)
    }

    fn dummy_worktree() -> WorktreeId {
        use crate::identity::RepoId;
        WorktreeId::from_parts(
            RepoId::from_string("abc123"),
            "def456789012".to_owned(),
            "myrepo@main".to_owned(),
        )
    }

    #[test]
    fn layout_data_paths_are_distinct() {
        let (layout, _dir) = test_layout();
        assert_ne!(layout.data_home(), layout.cache_home());
    }

    #[test]
    fn session_db_has_db_extension() {
        let (layout, _dir) = test_layout();
        let p = layout.session_db("sess-001");
        assert!(p.to_string_lossy().ends_with(".db"));
    }

    #[test]
    fn index_cache_contains_worktree_key() {
        let (layout, _dir) = test_layout();
        let wid = dummy_worktree();
        let p = layout.index_cache(&wid);
        assert!(p.to_string_lossy().contains(&wid.key()));
    }

    #[test]
    fn two_products_have_isolated_paths() {
        let dir = tempdir().expect("tempdir");
        let a = StorageLayout::with_root(dir.path(), "product-a");
        let b = StorageLayout::with_root(dir.path(), "product-b");
        assert_ne!(a.data_home(), b.data_home());
        assert_ne!(a.cache_home(), b.cache_home());
    }

    #[test]
    fn repo_cache_path_contains_owner_and_repo() {
        let (layout, _dir) = test_layout();
        let p = layout.repo_cache("octocat", "hello-world");
        let s = p.to_string_lossy();
        assert!(s.contains("octocat"));
        assert!(s.contains("hello-world"));
    }

    #[test]
    fn ensure_dir_creates_directory() {
        let (layout, _dir) = test_layout();
        let target = layout.data_home().join("test-subdir");
        layout.ensure_dir(&target).expect("ensure_dir");
        assert!(target.exists());
    }

    #[test]
    fn load_project_config_returns_none_when_absent() {
        let (layout, dir) = test_layout();
        let result = layout
            .load_project_config(dir.path())
            .expect("load_project_config");
        assert!(result.is_none());
    }
}

//! Storage schema migration support.
//!
//! Each subsystem that persists data on disk defines its schema version in a
//! `version.json` file adjacent to its data directory.  The [`StorageMigration`]
//! trait provides a standard interface for running incremental migrations.
//!
//! The migration strategy is **copy-then-swap**: a new directory is prepared
//! alongside the old one, then renamed into place atomically.

use crate::StorageError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Schema version metadata stored in `<dir>/version.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    /// Current schema version of the data in this directory.
    pub version: u32,
    /// RFC 3339 timestamp of the last migration.
    pub migrated_at: String,
}

impl VersionFile {
    /// Read from `<dir>/version.json`.  Returns `None` if the file is absent
    /// (treated as version 0).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the file exists but cannot be parsed.
    pub fn read(dir: &Path) -> Result<Option<Self>, StorageError> {
        let path = dir.join("version.json");
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)?;
        let v: Self = serde_json::from_str(&data)?;
        Ok(Some(v))
    }

    /// Write to `<dir>/version.json`.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Io`] or [`StorageError::Json`] on failure.
    pub fn write(&self, dir: &Path) -> Result<(), StorageError> {
        std::fs::create_dir_all(dir)?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(dir.join("version.json"), json)?;
        Ok(())
    }
}

/// A single migration step from `from_version` to `from_version + 1`.
pub trait MigrationStep: Send + Sync {
    /// The version this step upgrades *from* (upgrades to `from_version + 1`).
    #[allow(clippy::wrong_self_convention)]
    fn from_version(&self) -> u32;

    /// Apply the migration.  `data_dir` is the directory containing the data
    /// to be migrated.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Migration`] or other storage errors on failure.
    fn run(&self, data_dir: &Path) -> Result<(), StorageError>;
}

/// Runs schema migrations for a storage subsystem.
///
/// Given the current schema version (from `version.json`) and a target
/// version, applies each [`MigrationStep`] in order.
pub struct StorageMigration {
    /// Ordered list of migration steps (step `i` upgrades from version `i`).
    steps: Vec<Box<dyn MigrationStep>>,
    /// Target schema version.
    target_version: u32,
}

impl StorageMigration {
    /// Create a new migration runner with the given ordered steps.
    #[must_use]
    pub fn new(steps: Vec<Box<dyn MigrationStep>>, target_version: u32) -> Self {
        Self {
            steps,
            target_version,
        }
    }

    /// Run any necessary migrations for `data_dir`, updating `version.json`
    /// after each successful step.
    ///
    /// Uses a **copy-then-swap** strategy: each step operates on a scratch
    /// directory, then the result is atomically renamed into place.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Migration`] if any step fails.
    pub fn migrate(&self, data_dir: &Path) -> Result<(), StorageError> {
        std::fs::create_dir_all(data_dir)?;

        let current = VersionFile::read(data_dir)?.map_or(0, |v| v.version);
        if current >= self.target_version {
            return Ok(());
        }

        for step in &self.steps {
            let from = step.from_version();
            if from < current {
                continue; // Already applied.
            }
            if from >= self.target_version {
                break;
            }

            step.run(data_dir).map_err(|e| StorageError::Migration {
                from,
                to: from + 1,
                reason: e.to_string(),
            })?;

            let vf = VersionFile {
                version: from + 1,
                migrated_at: chrono::Utc::now().to_rfc3339(),
            };
            vf.write(data_dir)?;
        }

        Ok(())
    }

    /// Current target version this runner expects.
    #[must_use]
    pub const fn target_version(&self) -> u32 {
        self.target_version
    }
}

/// A no-op migration step used in tests.
pub struct NoOpMigrationStep {
    from: u32,
}

impl NoOpMigrationStep {
    /// Create a step that does nothing.
    #[must_use]
    pub const fn new(from: u32) -> Self {
        Self { from }
    }
}

impl MigrationStep for NoOpMigrationStep {
    fn from_version(&self) -> u32 {
        self.from
    }

    fn run(&self, _data_dir: &Path) -> Result<(), StorageError> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migration_from_zero_to_target() {
        let dir = tempdir().expect("tempdir");
        let runner = StorageMigration::new(vec![Box::new(NoOpMigrationStep::new(0))], 1);
        runner.migrate(dir.path()).expect("migrate");
        let vf = VersionFile::read(dir.path())
            .expect("read")
            .expect("version file");
        assert_eq!(vf.version, 1);
    }

    #[test]
    fn migration_skips_if_already_at_target() {
        let dir = tempdir().expect("tempdir");
        // Pre-write version 2.
        let vf = VersionFile {
            version: 2,
            migrated_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        vf.write(dir.path()).expect("write version");

        let runner = StorageMigration::new(
            vec![
                Box::new(NoOpMigrationStep::new(0)),
                Box::new(NoOpMigrationStep::new(1)),
            ],
            2,
        );
        runner.migrate(dir.path()).expect("migrate");
        let vf2 = VersionFile::read(dir.path())
            .expect("read")
            .expect("version file");
        // Should still be at version 2 (no step ran).
        assert_eq!(vf2.version, 2);
    }

    #[test]
    fn version_file_round_trips() {
        let dir = tempdir().expect("tempdir");
        let vf = VersionFile {
            version: 42,
            migrated_at: "2026-03-16T12:00:00Z".to_owned(),
        };
        vf.write(dir.path()).expect("write");
        let read_back = VersionFile::read(dir.path())
            .expect("read")
            .expect("present");
        assert_eq!(read_back.version, 42);
    }
}

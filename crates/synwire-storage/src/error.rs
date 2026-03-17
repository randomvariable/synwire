//! Error types for the storage layer.

use thiserror::Error;

/// Errors produced by the storage subsystem.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StorageError {
    /// An I/O error occurred while accessing the filesystem.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON serialisation or deserialisation error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// A `SQLite` error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// The Git first-commit hash could not be determined.
    #[error("Cannot determine Git first commit: {0}")]
    GitFirstCommit(String),

    /// A migration step failed.
    #[error("Migration failed (version {from} → {to}): {reason}")]
    Migration {
        /// Source schema version.
        from: u32,
        /// Target schema version.
        to: u32,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// The storage directory is not writable.
    #[error("Storage directory not writable: {path}")]
    NotWritable {
        /// Path that could not be written.
        path: String,
    },

    /// The config file is invalid.
    #[error("Invalid config at {path}: {reason}")]
    InvalidConfig {
        /// Path to the config file.
        path: String,
        /// Human-readable reason.
        reason: String,
    },

    /// An environment variable could not be read.
    #[error("Env var error: {0}")]
    Env(#[from] std::env::VarError),
}

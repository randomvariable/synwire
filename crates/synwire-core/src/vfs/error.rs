//! VFS error types.

use std::io;
use thiserror::Error;

/// VFS operation errors.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum VfsError {
    /// File or directory not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Permission denied.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Expected file, got directory.
    #[error("Is a directory: {0}")]
    IsDirectory(String),

    /// Path traversal attempt blocked.
    #[error("Path traversal blocked: attempted {attempted}, root {root}")]
    PathTraversal {
        /// Attempted path.
        attempted: String,
        /// Root path.
        root: String,
    },

    /// Operation outside allowed scope.
    #[error("Scope violation: path {path} outside scope {scope}")]
    ScopeViolation {
        /// Path that violated scope.
        path: String,
        /// Allowed scope.
        scope: String,
    },

    /// Resource limit exceeded.
    #[error("Resource limit: {0}")]
    ResourceLimit(String),

    /// Operation timed out.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// User denied approval.
    #[error("Operation denied: {0}")]
    OperationDenied(String),

    /// File was modified externally since last read.  Re-read before editing.
    #[error(
        "Stale read: {path} was modified externally since it was last read. Re-read the file before editing."
    )]
    StaleRead {
        /// Path of the stale file.
        path: String,
    },

    /// File has not been read yet.  Read before editing.
    #[error("Not read: {path} must be read before editing or writing. Read the file first.")]
    NotRead {
        /// Path that was not read.
        path: String,
    },

    /// Indexing denied for safety reasons.
    #[error("Index denied: {reason}")]
    IndexDenied {
        /// Reason for denial.
        reason: String,
    },

    /// Index is not ready yet — still building.
    #[error("Index not ready: {0}")]
    IndexNotReady(String),

    /// Provider doesn't support operation.
    #[error("Unsupported: {0}")]
    Unsupported(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

//! Native concurrency helpers for Synwire storage backends.
//!
//! Synwire uses native backend concurrency rather than external file locks:
//!
//! - **`SQLite`**: WAL mode (Write-Ahead Logging) for concurrent reads alongside
//!   writes without blocking.
//! - **Binary blobs**: atomic rename (`rename(2)`) for crash-safe updates.
//! - **`LanceDB` / tantivy**: leverage their own internal concurrency primitives.
//!
//! This module provides the helpers enforcing these conventions.

use crate::StorageError;
use rusqlite::Connection;
use std::io;
use std::path::Path;

/// Enable WAL mode on the given `SQLite` connection.
///
/// WAL mode allows concurrent reads and a single writer without the writer
/// blocking readers (unlike the default journal mode).  Must be called once
/// per connection before any writes.
///
/// # Errors
///
/// Returns [`StorageError::Sqlite`] if the `PRAGMA journal_mode=WAL` command
/// fails.
pub fn ensure_wal_mode(conn: &Connection) -> Result<(), StorageError> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

/// Atomically replace `dest` with the contents of `src` using a rename.
///
/// Writes to a temporary file adjacent to `dest`, then renames it into place.
/// Because `rename(2)` is atomic on POSIX systems (and atomic at the
/// filesystem level on Windows for files on the same volume), this prevents
/// readers from observing a partially written file.
///
/// # Errors
///
/// Returns an I/O error if the source cannot be read, the temp file cannot be
/// written, or the rename fails.
pub fn atomic_write(dest: &Path, data: &[u8]) -> io::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "dest has no parent directory")
    })?;
    // Write to a sibling temp file.
    let tmp_path = parent.join(format!(
        ".tmp-{}",
        dest.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("atomic")
    ));
    std::fs::write(&tmp_path, data)?;
    // Atomically replace the destination.
    std::fs::rename(&tmp_path, dest)
}

/// Open a `SQLite` database in WAL mode, creating it (and its parent
/// directories) if it does not exist.
///
/// # Errors
///
/// Returns [`StorageError`] if the directory cannot be created or the
/// database cannot be opened/configured.
pub fn open_wal_database(path: &Path) -> Result<Connection, StorageError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    ensure_wal_mode(&conn)?;
    Ok(conn)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn wal_mode_is_set() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let conn = open_wal_database(&db_path).expect("open_wal_database");
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .expect("PRAGMA journal_mode");
        assert_eq!(mode, "wal");
    }

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempdir().expect("tempdir");
        let dest = dir.path().join("output.bin");
        atomic_write(&dest, b"hello world").expect("atomic_write");
        let contents = std::fs::read(&dest).expect("read");
        assert_eq!(contents, b"hello world");
    }

    #[test]
    fn atomic_write_replaces_existing() {
        let dir = tempdir().expect("tempdir");
        let dest = dir.path().join("output.bin");
        atomic_write(&dest, b"v1").expect("atomic_write v1");
        atomic_write(&dest, b"v2").expect("atomic_write v2");
        let contents = std::fs::read(&dest).expect("read");
        assert_eq!(contents, b"v2");
    }
}

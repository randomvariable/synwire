//! SQLite-backed experience pool with global + project-local tiers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

/// Error type for experience pool operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ExperienceError {
    /// `SQLite` error.
    #[error("SQLite error: {0}")]
    Sqlite(String),
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<rusqlite::Error> for ExperienceError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e.to_string())
    }
}

impl From<std::io::Error> for ExperienceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// A single experience entry linking a task description to modified files.
#[non_exhaustive]
pub struct ExperienceEntry {
    /// Task description (from the agent prompt).
    pub task_description: String,
    /// Files modified in this edit session.
    pub files_modified: Vec<String>,
    /// Timestamp (ISO 8601).
    pub recorded_at: String,
}

/// Stop words to skip when tokenising a task description for keyword matching.
const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "from", "are", "was", "were", "have", "has",
    "been", "will", "would", "could", "should", "into", "onto", "over", "under", "also", "then",
    "than", "when",
];

fn keywords(description: &str) -> Vec<String> {
    description
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3)
        .map(str::to_lowercase)
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .collect()
}

fn init_schema(conn: &rusqlite::Connection) -> Result<(), ExperienceError> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS experiences (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             task_description TEXT NOT NULL,
             file_path TEXT NOT NULL,
             recorded_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_task ON experiences(task_description);
         CREATE INDEX IF NOT EXISTS idx_file ON experiences(file_path);",
    )?;
    Ok(())
}

/// SQLite-backed experience pool.
///
/// Records task-to-file associations and supports keyword-based file retrieval.
pub struct ExperiencePool {
    conn: Mutex<rusqlite::Connection>,
}

impl ExperiencePool {
    /// Open or create the experience database at the given path.
    pub fn open(path: &Path) -> Result<Self, ExperienceError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = rusqlite::Connection::open(path)?;
        init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Record an edit event, inserting one row per file in [`ExperienceEntry::files_modified`].
    #[allow(clippy::significant_drop_tightening)]
    pub fn record(&self, entry: &ExperienceEntry) -> Result<(), ExperienceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ExperienceError::Sqlite(e.to_string()))?;
        for file in &entry.files_modified {
            let _rows_inserted = conn.execute(
                "INSERT INTO experiences (task_description, file_path, recorded_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![entry.task_description, file, entry.recorded_at],
            )?;
        }
        Ok(())
    }

    /// Query relevant files for a task description.
    ///
    /// Uses keyword matching against stored task descriptions.
    /// Returns `(file_path, count)` pairs sorted by frequency descending.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use synwire_agent::experience::{ExperiencePool, ExperienceEntry};
    /// # let pool = ExperiencePool::open(std::path::Path::new("/tmp/exp.db")).unwrap();
    /// let files = pool.query_files("fix authentication bug").unwrap();
    /// for (path, count) in files {
    ///     println!("{path}: {count}");
    /// }
    /// ```
    #[allow(clippy::significant_drop_tightening)]
    pub fn query_files(&self, description: &str) -> Result<Vec<(String, u32)>, ExperienceError> {
        let words = keywords(description);
        if words.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| ExperienceError::Sqlite(e.to_string()))?;
        let mut totals: HashMap<String, u32> = HashMap::new();

        for keyword in &words {
            let pattern = format!("%{keyword}%");
            let mut stmt = conn.prepare(
                "SELECT file_path, COUNT(*) as cnt FROM experiences
                 WHERE task_description LIKE ?1
                 GROUP BY file_path",
            )?;
            let rows = stmt.query_map(rusqlite::params![pattern], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
            })?;
            for row in rows {
                let (file, cnt) = row?;
                *totals.entry(file).or_insert(0) += cnt;
            }
        }

        let mut result: Vec<(String, u32)> = totals.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(result)
    }
}

/// Two-tier experience pool: project-local first, global fallback.
///
/// Queries local pool first; falls back to global when local yields no results.
/// Records to both tiers on every edit event.
pub struct TieredExperiencePool {
    local: ExperiencePool,
    global: ExperiencePool,
}

impl TieredExperiencePool {
    /// Open both local and global experience pool tiers.
    pub fn open(local_path: &Path, global_path: &Path) -> Result<Self, ExperienceError> {
        Ok(Self {
            local: ExperiencePool::open(local_path)?,
            global: ExperiencePool::open(global_path)?,
        })
    }

    /// Query with local-first fallback to global.
    ///
    /// Returns local results if any exist; otherwise returns global results.
    pub fn query_files(&self, description: &str) -> Result<Vec<(String, u32)>, ExperienceError> {
        let local = self.local.query_files(description)?;
        if !local.is_empty() {
            return Ok(local);
        }
        self.global.query_files(description)
    }

    /// Record to both local and global pools.
    pub fn record(&self, entry: &ExperienceEntry) -> Result<(), ExperienceError> {
        self.local.record(entry)?;
        self.global.record(entry)?;
        Ok(())
    }
}

/// Record an edit completion event to the experience pool.
///
/// Called by the agent runtime after a successful edit directive completes.
///
/// # Errors
///
/// Returns [`ExperienceError`] if the database write fails.
pub fn record_edit_completion(
    pool: &ExperiencePool,
    description: &str,
    files: &[&str],
) -> Result<(), ExperienceError> {
    let entry = ExperienceEntry {
        task_description: description.to_owned(),
        files_modified: files.iter().map(|s| (*s).to_string()).collect(),
        recorded_at: chrono::Utc::now().to_rfc3339(),
    };
    pool.record(&entry)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn experience_pool_cross_session() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ExperiencePool::open(dir.path().join("exp.db").as_path()).unwrap();

        let entry = ExperienceEntry {
            task_description: "fix authentication bug".to_owned(),
            files_modified: vec!["src/auth.rs".to_owned(), "src/middleware.rs".to_owned()],
            recorded_at: "2024-01-01T00:00:00Z".to_owned(),
        };
        pool.record(&entry).unwrap();

        let files = pool.query_files("authentication").unwrap();
        assert!(!files.is_empty());
        assert!(files.iter().any(|(f, _)| f.contains("auth")));
    }

    #[test]
    fn tiered_pool_falls_back_to_global() {
        let dir = tempfile::tempdir().unwrap();
        let local_path = dir.path().join("local.db");
        let global_path = dir.path().join("global.db");
        let tiered = TieredExperiencePool::open(&local_path, &global_path).unwrap();

        // Record only to global via the underlying global pool
        let global = ExperiencePool::open(&global_path).unwrap();
        let entry = ExperienceEntry {
            task_description: "network timeout handling".to_owned(),
            files_modified: vec!["src/network.rs".to_owned()],
            recorded_at: "2024-01-01T00:00:00Z".to_owned(),
        };
        global.record(&entry).unwrap();

        // Local yields nothing; global fallback should fire
        let files = tiered.query_files("network timeout").unwrap();
        assert!(!files.is_empty());
    }

    #[test]
    fn record_edit_completion_helper() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ExperiencePool::open(dir.path().join("exp.db").as_path()).unwrap();
        record_edit_completion(&pool, "refactor parser logic", &["src/parser.rs"]).unwrap();
        let files = pool.query_files("parser logic").unwrap();
        assert!(!files.is_empty());
    }
}

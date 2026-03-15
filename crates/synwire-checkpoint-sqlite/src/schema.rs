//! Database schema for the `SQLite` checkpoint backend.

/// SQL statement to create the checkpoints table.
pub const CREATE_CHECKPOINTS_TABLE: &str = r"
CREATE TABLE IF NOT EXISTS checkpoints (
    thread_id TEXT NOT NULL,
    checkpoint_id TEXT NOT NULL,
    data BLOB NOT NULL,
    metadata TEXT NOT NULL,
    parent_checkpoint_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (thread_id, checkpoint_id)
)
";

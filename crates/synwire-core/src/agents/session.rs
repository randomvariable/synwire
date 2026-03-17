//! Session management traits.
//!
//! A session captures the complete state of an agent conversation so it can
//! be resumed, forked, rewound, or archived across process restarts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::BoxFuture;
use crate::agents::error::AgentError;

// ---------------------------------------------------------------------------
// Session metadata
// ---------------------------------------------------------------------------

/// Metadata attached to a stored session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Unique session identifier.
    pub id: String,
    /// Human-readable name.
    pub name: Option<String>,
    /// Arbitrary user-defined tags.
    pub tags: Vec<String>,
    /// Agent name this session belongs to.
    pub agent_name: String,
    /// Creation timestamp (Unix seconds).
    pub created_at: i64,
    /// Last-updated timestamp (Unix seconds).
    pub updated_at: i64,
    /// Number of turns recorded.
    pub turn_count: u32,
    /// Cumulative token usage.
    pub total_tokens: u64,
}

/// Full session snapshot including conversation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session metadata.
    pub metadata: SessionMetadata,
    /// Conversation messages as JSON array.
    pub messages: Vec<Value>,
    /// Arbitrary agent state (plugin state, cwd, env, etc.).
    pub state: Value,
}

// ---------------------------------------------------------------------------
// SessionManager trait
// ---------------------------------------------------------------------------

/// Manages session persistence and lifecycle operations.
///
/// All operations are async and return `AgentError` on failure.
pub trait SessionManager: Send + Sync {
    /// List all sessions, ordered by most-recently updated first.
    fn list(&self) -> BoxFuture<'_, Result<Vec<SessionMetadata>, AgentError>>;

    /// Load a session by ID.
    fn resume(&self, session_id: &str) -> BoxFuture<'_, Result<Session, AgentError>>;

    /// Save (create or update) a session.
    fn save(&self, session: &Session) -> BoxFuture<'_, Result<(), AgentError>>;

    /// Delete a session and all associated data.
    fn delete(&self, session_id: &str) -> BoxFuture<'_, Result<(), AgentError>>;

    /// Fork a session — create a copy with a new ID.
    ///
    /// The forked session shares the same history up to the fork point but
    /// accumulates diverging state independently thereafter.
    fn fork(
        &self,
        session_id: &str,
        new_name: Option<String>,
    ) -> BoxFuture<'_, Result<SessionMetadata, AgentError>>;

    /// Rewind a session to a previous turn.
    ///
    /// `turn_index` is zero-based.  Messages beyond `turn_index` are discarded.
    fn rewind(
        &self,
        session_id: &str,
        turn_index: u32,
    ) -> BoxFuture<'_, Result<Session, AgentError>>;

    /// Add one or more tags to a session.
    fn tag(&self, session_id: &str, tags: Vec<String>) -> BoxFuture<'_, Result<(), AgentError>>;

    /// Rename a session.
    fn rename(&self, session_id: &str, new_name: String) -> BoxFuture<'_, Result<(), AgentError>>;
}

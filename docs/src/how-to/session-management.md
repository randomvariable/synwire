# How to: Manage Agent Sessions

**Goal:** Create, persist, resume, fork, rewind, tag, and delete agent sessions using `SessionManager`.

---

## Core types

```rust
pub struct SessionMetadata {
    pub id: String,           // UUID
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub agent_name: String,
    pub created_at: i64,      // Unix milliseconds
    pub updated_at: i64,      // Unix milliseconds
    pub turn_count: u32,
    pub total_tokens: u64,
}

pub struct Session {
    pub metadata: SessionMetadata,
    pub messages: Vec<serde_json::Value>,  // conversation history
    pub state: serde_json::Value,          // arbitrary agent state
}
```

---

## InMemorySessionManager

The built-in implementation. All data is lost when the process exits. Use it for tests or ephemeral agents; use the checkpoint-backed implementation for persistence.

```rust
use synwire_agent::session::manager::InMemorySessionManager;
use synwire_core::agents::session::{Session, SessionManager, SessionMetadata};

let mgr = InMemorySessionManager::new();
```

---

## Save and resume

```rust
use serde_json::json;

// Construct a new session.
let session = Session {
    metadata: SessionMetadata {
        id: uuid::Uuid::new_v4().to_string(),
        name: Some("my-task".to_string()),
        tags: vec!["production".to_string()],
        agent_name: "code-agent".to_string(),
        created_at: 0,
        updated_at: 0,
        turn_count: 0,
        total_tokens: 0,
    },
    messages: Vec::new(),
    state: json!({"cwd": "/home/user"}),
};

mgr.save(&session).await?;

// Later — in the same or a new call — resume by ID.
let loaded = mgr.resume(&session.metadata.id).await?;
```

`save` sets `updated_at` to the current time. Calling `save` on an existing ID updates it in place.

---

## List sessions

Returns all sessions ordered by `updated_at` descending (most recently active first).

```rust
let sessions: Vec<SessionMetadata> = mgr.list().await?;
for s in &sessions {
    println!("{} {:?} turns={}", s.id, s.name, s.turn_count);
}
```

---

## Fork a session

Creates an independent copy with a new UUID. The fork shares history up to the fork point but diverges independently afterwards.

```rust
let fork_meta = mgr.fork(&session.metadata.id, Some("experiment-branch".to_string())).await?;
assert_ne!(fork_meta.id, session.metadata.id);
```

Pass `None` as the name to auto-generate a name of the form `"<original name> (fork)"`.

---

## Rewind to a previous turn

Truncates the message list to `turn_index` entries (zero-based). Useful for re-running from a specific point without forking.

```rust
// Keep only the first 3 messages.
let rewound = mgr.rewind(&session.metadata.id, 3).await?;
assert_eq!(rewound.messages.len(), 3);
```

---

## Tag a session

Adds one or more tags without duplicates. Existing tags are preserved.

```rust
mgr.tag(&session.metadata.id, vec!["reviewed".to_string(), "archived".to_string()]).await?;
```

---

## Rename a session

```rust
mgr.rename(&session.metadata.id, "final-delivery".to_string()).await?;
```

---

## Delete a session

```rust
mgr.delete(&session.metadata.id).await?;
// Subsequent resume returns AgentError::Session.
```

---

## Implementing SessionManager

To persist sessions to a database or remote store, implement the `SessionManager` trait. All methods return `BoxFuture` so async storage drivers are supported.

```rust
use synwire_core::agents::session::{Session, SessionManager, SessionMetadata};
use synwire_core::agents::error::AgentError;
use synwire_core::BoxFuture;

struct RedisSessionManager { /* ... */ }

impl SessionManager for RedisSessionManager {
    fn list(&self) -> BoxFuture<'_, Result<Vec<SessionMetadata>, AgentError>> {
        Box::pin(async move { /* query Redis ZREVRANGE */ todo!() })
    }

    fn resume(&self, session_id: &str) -> BoxFuture<'_, Result<Session, AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move { /* GET id */ todo!() })
    }

    fn save(&self, session: &Session) -> BoxFuture<'_, Result<(), AgentError>> {
        let session = session.clone();
        Box::pin(async move { /* SET id */ todo!() })
    }

    fn delete(&self, session_id: &str) -> BoxFuture<'_, Result<(), AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move { /* DEL id */ todo!() })
    }

    fn fork(&self, session_id: &str, new_name: Option<String>) -> BoxFuture<'_, Result<SessionMetadata, AgentError>> {
        todo!()
    }

    fn rewind(&self, session_id: &str, turn_index: u32) -> BoxFuture<'_, Result<Session, AgentError>> {
        todo!()
    }

    fn tag(&self, session_id: &str, tags: Vec<String>) -> BoxFuture<'_, Result<(), AgentError>> {
        todo!()
    }

    fn rename(&self, session_id: &str, new_name: String) -> BoxFuture<'_, Result<(), AgentError>> {
        todo!()
    }
}
```

---

**See also**

- [How to: Add Checkpointing](add-checkpointing.md)
- [Explanation: Architecture](../explanation/architecture.md)

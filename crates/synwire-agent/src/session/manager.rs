//! In-memory `SessionManager` implementation.
//!
//! Suitable for ephemeral use and testing.  A checkpoint-backed implementation
//! lives in `synwire-checkpoint`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::agents::session::{Session, SessionManager, SessionMetadata};

fn now_unix() -> i64 {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(ms).unwrap_or(i64::MAX)
}

/// In-memory session manager.  All data is lost when the process exits.
#[derive(Debug, Default)]
pub struct InMemorySessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl InMemorySessionManager {
    /// Create an empty manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionManager for InMemorySessionManager {
    fn list(&self) -> BoxFuture<'_, Result<Vec<SessionMetadata>, AgentError>> {
        Box::pin(async move {
            let mut metas: Vec<SessionMetadata> = self
                .sessions
                .read()
                .await
                .values()
                .map(|s| s.metadata.clone())
                .collect();
            metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(metas)
        })
    }

    fn resume(&self, session_id: &str) -> BoxFuture<'_, Result<Session, AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move {
            self.sessions
                .read()
                .await
                .get(&id)
                .cloned()
                .ok_or_else(|| AgentError::Session(format!("Session not found: {id}")))
        })
    }

    fn save(&self, session: &Session) -> BoxFuture<'_, Result<(), AgentError>> {
        let mut session = session.clone();
        session.metadata.updated_at = now_unix();
        Box::pin(async move {
            let _ = self
                .sessions
                .write()
                .await
                .insert(session.metadata.id.clone(), session);
            Ok(())
        })
    }

    fn delete(&self, session_id: &str) -> BoxFuture<'_, Result<(), AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move {
            let _ = self.sessions.write().await.remove(&id).ok_or_else(|| {
                AgentError::Session(format!("Session not found for deletion: {id}"))
            })?;
            Ok(())
        })
    }

    fn fork(
        &self,
        session_id: &str,
        new_name: Option<String>,
    ) -> BoxFuture<'_, Result<SessionMetadata, AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move {
            let source = self
                .sessions
                .read()
                .await
                .get(&id)
                .cloned()
                .ok_or_else(|| AgentError::Session(format!("Session not found: {id}")))?;

            let new_id = Uuid::new_v4().to_string();
            let now = now_unix();
            let mut forked = source.clone();
            forked.metadata.id = new_id.clone();
            forked.metadata.name = new_name.or_else(|| {
                source
                    .metadata
                    .name
                    .as_deref()
                    .map(|n| format!("{n} (fork)"))
            });
            forked.metadata.created_at = now;
            forked.metadata.updated_at = now;

            let meta = forked.metadata.clone();
            let _ = self.sessions.write().await.insert(new_id, forked);
            Ok(meta)
        })
    }

    fn rewind(
        &self,
        session_id: &str,
        turn_index: u32,
    ) -> BoxFuture<'_, Result<Session, AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(&id)
                .ok_or_else(|| AgentError::Session(format!("Session not found: {id}")))?;
            let max = session.messages.len();
            let keep = (turn_index as usize).min(max);
            session.messages.truncate(keep);
            session.metadata.turn_count = u32::try_from(keep).unwrap_or(u32::MAX);
            session.metadata.updated_at = now_unix();
            let result = session.clone();
            drop(guard);
            Ok(result)
        })
    }

    fn tag(&self, session_id: &str, tags: Vec<String>) -> BoxFuture<'_, Result<(), AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(&id)
                .ok_or_else(|| AgentError::Session(format!("Session not found: {id}")))?;
            for tag in tags {
                if !session.metadata.tags.contains(&tag) {
                    session.metadata.tags.push(tag);
                }
            }
            session.metadata.updated_at = now_unix();
            drop(guard);
            Ok(())
        })
    }

    fn rename(&self, session_id: &str, new_name: String) -> BoxFuture<'_, Result<(), AgentError>> {
        let id = session_id.to_string();
        Box::pin(async move {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(&id)
                .ok_or_else(|| AgentError::Session(format!("Session not found: {id}")))?;
            session.metadata.name = Some(new_name);
            session.metadata.updated_at = now_unix();
            drop(guard);
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use synwire_core::agents::session::Session;

    fn make_session(id: &str, agent: &str) -> Session {
        Session {
            metadata: SessionMetadata {
                id: id.to_string(),
                name: None,
                tags: Vec::new(),
                agent_name: agent.to_string(),
                created_at: now_unix(),
                updated_at: now_unix(),
                turn_count: 0,
                total_tokens: 0,
            },
            messages: Vec::new(),
            state: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_save_and_resume() {
        let mgr = InMemorySessionManager::new();
        let session = make_session("s1", "agent-a");
        mgr.save(&session).await.unwrap();

        let loaded = mgr.resume("s1").await.unwrap();
        assert_eq!(loaded.metadata.id, "s1");
    }

    #[tokio::test]
    async fn test_list_ordered_by_updated() {
        let mgr = InMemorySessionManager::new();
        mgr.save(&make_session("a", "x")).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        mgr.save(&make_session("b", "x")).await.unwrap();

        let list = mgr.list().await.unwrap();
        // "b" has a larger updated_at.
        assert_eq!(list[0].id, "b");
    }

    #[tokio::test]
    async fn test_fork() {
        let mgr = InMemorySessionManager::new();
        mgr.save(&make_session("orig", "agent")).await.unwrap();
        let forked = mgr.fork("orig", Some("fork-1".to_string())).await.unwrap();
        assert_ne!(forked.id, "orig");
        assert_eq!(forked.name.as_deref(), Some("fork-1"));
    }

    #[tokio::test]
    async fn test_rewind() {
        let mgr = InMemorySessionManager::new();
        let mut s = make_session("r1", "agent");
        s.messages = vec![
            serde_json::json!({"role": "user", "content": "a"}),
            serde_json::json!({"role": "assistant", "content": "b"}),
            serde_json::json!({"role": "user", "content": "c"}),
        ];
        mgr.save(&s).await.unwrap();

        let rewound = mgr.rewind("r1", 1).await.unwrap();
        assert_eq!(rewound.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_tag_and_rename() {
        let mgr = InMemorySessionManager::new();
        mgr.save(&make_session("t1", "agent")).await.unwrap();
        mgr.tag("t1", vec!["important".to_string()]).await.unwrap();
        mgr.rename("t1", "My Session".to_string()).await.unwrap();

        let s = mgr.resume("t1").await.unwrap();
        assert!(s.metadata.tags.contains(&"important".to_string()));
        assert_eq!(s.metadata.name.as_deref(), Some("My Session"));
    }

    #[tokio::test]
    async fn test_delete() {
        let mgr = InMemorySessionManager::new();
        mgr.save(&make_session("del", "agent")).await.unwrap();
        mgr.delete("del").await.unwrap();
        assert!(mgr.resume("del").await.is_err());
    }
}

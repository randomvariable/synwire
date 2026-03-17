//! `SessionManager` conformance test harness.

#![allow(clippy::expect_used)]

use serde_json::Value;
use synwire_core::agents::session::{Session, SessionManager, SessionMetadata};

fn make_session(id: &str) -> Session {
    Session {
        metadata: SessionMetadata {
            id: id.to_string(),
            name: Some(format!("Test Session {id}")),
            tags: Vec::new(),
            agent_name: "conformance-agent".to_string(),
            created_at: 0,
            updated_at: 0,
            turn_count: 0,
            total_tokens: 0,
        },
        messages: Vec::new(),
        state: Value::Object(serde_json::Map::default()),
    }
}

/// Run the full `SessionManager` conformance suite against `mgr`.
///
/// # Panics
/// Panics with a descriptive message if any assertion fails.
pub async fn run_session_conformance(mgr: &(impl SessionManager + ?Sized)) {
    test_save_resume(mgr).await;
    test_list(mgr).await;
    test_delete(mgr).await;
    test_fork(mgr).await;
    test_rewind(mgr).await;
    test_tag(mgr).await;
    test_rename(mgr).await;
    test_resume_missing(mgr).await;
}

async fn test_save_resume(mgr: &(impl SessionManager + ?Sized)) {
    let s = make_session("conf-sr");
    mgr.save(&s).await.expect("save should succeed");
    let loaded = mgr.resume("conf-sr").await.expect("resume should succeed");
    assert_eq!(loaded.metadata.id, "conf-sr");
}

async fn test_list(mgr: &(impl SessionManager + ?Sized)) {
    let s = make_session("conf-list");
    mgr.save(&s).await.expect("save");
    let list = mgr.list().await.expect("list");
    assert!(
        list.iter().any(|m| m.id == "conf-list"),
        "list must include saved session"
    );
}

async fn test_delete(mgr: &(impl SessionManager + ?Sized)) {
    let s = make_session("conf-del");
    mgr.save(&s).await.expect("save");
    mgr.delete("conf-del").await.expect("delete");
    assert!(
        mgr.resume("conf-del").await.is_err(),
        "resume after delete must fail"
    );
}

async fn test_fork(mgr: &(impl SessionManager + ?Sized)) {
    let s = make_session("conf-fork-src");
    mgr.save(&s).await.expect("save source");
    let forked = mgr
        .fork("conf-fork-src", Some("forked".to_string()))
        .await
        .expect("fork");
    assert_ne!(forked.id, "conf-fork-src", "fork must have a new ID");
    assert_eq!(forked.name.as_deref(), Some("forked"));
}

async fn test_rewind(mgr: &(impl SessionManager + ?Sized)) {
    let mut s = make_session("conf-rewind");
    s.messages = vec![
        serde_json::json!({"role": "user", "content": "a"}),
        serde_json::json!({"role": "assistant", "content": "b"}),
        serde_json::json!({"role": "user", "content": "c"}),
    ];
    mgr.save(&s).await.expect("save");
    let rewound = mgr.rewind("conf-rewind", 1).await.expect("rewind");
    assert_eq!(
        rewound.messages.len(),
        1,
        "rewind to turn 1 should keep 1 message"
    );
}

async fn test_tag(mgr: &(impl SessionManager + ?Sized)) {
    let s = make_session("conf-tag");
    mgr.save(&s).await.expect("save");
    mgr.tag("conf-tag", vec!["important".to_string()])
        .await
        .expect("tag");
    let loaded = mgr.resume("conf-tag").await.expect("resume");
    assert!(
        loaded.metadata.tags.contains(&"important".to_string()),
        "tag must be persisted"
    );
}

async fn test_rename(mgr: &(impl SessionManager + ?Sized)) {
    let s = make_session("conf-rename");
    mgr.save(&s).await.expect("save");
    mgr.rename("conf-rename", "New Name".to_string())
        .await
        .expect("rename");
    let loaded = mgr.resume("conf-rename").await.expect("resume");
    assert_eq!(
        loaded.metadata.name.as_deref(),
        Some("New Name"),
        "rename must persist"
    );
}

async fn test_resume_missing(mgr: &(impl SessionManager + ?Sized)) {
    let result = mgr.resume("nonexistent-session-id").await;
    assert!(result.is_err(), "resume of missing session must fail");
}

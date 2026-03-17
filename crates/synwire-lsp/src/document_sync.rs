//! Document lifecycle tracking for `didOpen`/`didChange`/`didClose`.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lsp_types::Url;

/// Tracks which documents are open and their current version/content.
#[derive(Debug, Clone)]
pub struct DocumentSyncManager {
    open_documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
    max_cache_size: usize,
}

/// Internal state for a single open document.
#[derive(Debug, Clone)]
pub(crate) struct DocumentState {
    /// The language identifier (e.g. `"rust"`, `"python"`).
    pub language_id: String,
    /// Monotonically increasing version counter.
    pub version: i32,
    /// Full text content of the document.
    pub content: String,
}

impl DocumentSyncManager {
    /// Create a new manager with the given maximum cache size.
    #[must_use]
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            open_documents: Arc::new(RwLock::new(HashMap::new())),
            max_cache_size,
        }
    }

    /// Record that a document was opened.
    ///
    /// Returns `false` if the cache is full and the document was not added.
    pub fn open(&self, uri: Url, language_id: String, content: String) -> bool {
        if let Ok(mut docs) = self.open_documents.write() {
            if docs.len() >= self.max_cache_size && !docs.contains_key(&uri) {
                return false;
            }
            let _prev = docs.insert(
                uri,
                DocumentState {
                    language_id,
                    version: 0,
                    content,
                },
            );
            true
        } else {
            false
        }
    }

    /// Record that a document's content changed. Bumps the version counter.
    ///
    /// Returns the new version, or `None` if the document is not tracked.
    pub fn change(&self, uri: &Url, new_content: String) -> Option<i32> {
        if let Ok(mut docs) = self.open_documents.write() {
            if let Some(state) = docs.get_mut(uri) {
                state.version += 1;
                state.content = new_content;
                return Some(state.version);
            }
        }
        None
    }

    /// Record that a document was closed.
    pub fn close(&self, uri: &Url) {
        if let Ok(mut docs) = self.open_documents.write() {
            let _removed = docs.remove(uri);
        }
    }

    /// Check whether a document is currently open.
    #[must_use]
    pub fn is_open(&self, uri: &Url) -> bool {
        self.open_documents
            .read()
            .is_ok_and(|docs| docs.contains_key(uri))
    }

    /// Get the current version of an open document.
    #[must_use]
    pub fn version(&self, uri: &Url) -> Option<i32> {
        self.open_documents
            .read()
            .ok()
            .and_then(|docs| docs.get(uri).map(|s| s.version))
    }

    /// Get the current content of an open document.
    #[must_use]
    pub fn content(&self, uri: &Url) -> Option<String> {
        self.open_documents
            .read()
            .ok()
            .and_then(|docs| docs.get(uri).map(|s| s.content.clone()))
    }

    /// Get the language id of an open document.
    #[must_use]
    pub fn language_id(&self, uri: &Url) -> Option<String> {
        self.open_documents
            .read()
            .ok()
            .and_then(|docs| docs.get(uri).map(|s| s.language_id.clone()))
    }

    /// Number of currently open documents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.open_documents
            .read()
            .map(|docs| docs.len())
            .unwrap_or(0)
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::manual_string_new)]
mod tests {
    use super::*;

    fn test_uri(path: &str) -> Url {
        Url::parse(&format!("file://{path}")).unwrap()
    }

    #[test]
    fn open_and_query() {
        let mgr = DocumentSyncManager::new(10);
        let uri = test_uri("/tmp/test.rs");
        assert!(mgr.open(uri.clone(), "rust".into(), "fn main() {}".into()));
        assert!(mgr.is_open(&uri));
        assert_eq!(mgr.version(&uri), Some(0));
        assert_eq!(mgr.content(&uri).as_deref(), Some("fn main() {}"));
        assert_eq!(mgr.language_id(&uri).as_deref(), Some("rust"));
    }

    #[test]
    fn change_bumps_version() {
        let mgr = DocumentSyncManager::new(10);
        let uri = test_uri("/tmp/test.rs");
        let _ = mgr.open(uri.clone(), "rust".into(), "v0".into());
        assert_eq!(mgr.change(&uri, "v1".into()), Some(1));
        assert_eq!(mgr.change(&uri, "v2".into()), Some(2));
        assert_eq!(mgr.content(&uri).as_deref(), Some("v2"));
    }

    #[test]
    fn close_removes() {
        let mgr = DocumentSyncManager::new(10);
        let uri = test_uri("/tmp/test.rs");
        let _ = mgr.open(uri.clone(), "rust".into(), "".into());
        mgr.close(&uri);
        assert!(!mgr.is_open(&uri));
        assert_eq!(mgr.version(&uri), None);
    }

    #[test]
    fn respects_max_cache_size() {
        let mgr = DocumentSyncManager::new(1);
        let uri1 = test_uri("/tmp/a.rs");
        let uri2 = test_uri("/tmp/b.rs");
        assert!(mgr.open(uri1, "rust".into(), "".into()));
        assert!(!mgr.open(uri2, "rust".into(), "".into()));
    }
}

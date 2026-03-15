//! In-memory implementation of `BaseStore`.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::types::CheckpointError;

use super::base::{BaseStore, Item, SearchItem};

/// An in-memory key-value store backed by a `RwLock<HashMap>`.
///
/// Suitable for testing and development. Data is lost when the process exits.
#[derive(Debug, Clone, Default)]
pub struct InMemoryStore {
    data: Arc<RwLock<HashMap<String, HashMap<String, Item>>>>,
}

impl InMemoryStore {
    /// Create a new, empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[allow(clippy::significant_drop_tightening)]
impl BaseStore for InMemoryStore {
    fn get<'a>(
        &'a self,
        namespace: &'a str,
        key: &'a str,
    ) -> synwire_core::BoxFuture<'a, Result<Option<Item>, CheckpointError>> {
        Box::pin(async move {
            let data = self.data.read().await;
            Ok(data.get(namespace).and_then(|ns| ns.get(key)).cloned())
        })
    }

    fn put<'a>(
        &'a self,
        namespace: &'a str,
        key: &'a str,
        value: serde_json::Value,
    ) -> synwire_core::BoxFuture<'a, Result<(), CheckpointError>> {
        Box::pin(async move {
            let now = Utc::now();
            let mut data = self.data.write().await;
            let ns = data.entry(namespace.to_owned()).or_default();
            match ns.get_mut(key) {
                Some(item) => {
                    item.value = value;
                    item.updated_at = now;
                }
                None => {
                    let _prev = ns.insert(
                        key.to_owned(),
                        Item {
                            namespace: namespace.to_owned(),
                            key: key.to_owned(),
                            value,
                            created_at: now,
                            updated_at: now,
                        },
                    );
                }
            }
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        namespace: &'a str,
        key: &'a str,
    ) -> synwire_core::BoxFuture<'a, Result<(), CheckpointError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if let Some(ns) = data.get_mut(namespace) {
                let _removed = ns.remove(key);
            }
            Ok(())
        })
    }

    fn search<'a>(
        &'a self,
        namespace: &'a str,
        query: Option<&'a str>,
        limit: usize,
    ) -> synwire_core::BoxFuture<'a, Result<Vec<SearchItem>, CheckpointError>> {
        Box::pin(async move {
            let data = self.data.read().await;
            let Some(ns) = data.get(namespace) else {
                return Ok(Vec::new());
            };
            let items: Vec<SearchItem> = ns
                .values()
                .filter(|item| {
                    query.is_none_or(|q| item.key.contains(q) || item.value.to_string().contains(q))
                })
                .take(limit)
                .map(|item| SearchItem {
                    item: item.clone(),
                    score: 1.0,
                })
                .collect();
            Ok(items)
        })
    }

    fn list_namespaces(&self) -> synwire_core::BoxFuture<'_, Result<Vec<String>, CheckpointError>> {
        Box::pin(async move {
            let namespaces = self.data.read().await.keys().cloned().collect();
            Ok(namespaces)
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    /// T220: `InMemoryStore` CRUD operations.
    #[tokio::test]
    async fn crud_operations() {
        let store = InMemoryStore::new();

        // Put
        store.put("ns1", "key1", json!({"data": 1})).await.unwrap();

        // Get
        let item = store.get("ns1", "key1").await.unwrap().unwrap();
        assert_eq!(item.value, json!({"data": 1}));
        assert_eq!(item.namespace, "ns1");
        assert_eq!(item.key, "key1");

        // Update
        store.put("ns1", "key1", json!({"data": 2})).await.unwrap();
        let item = store.get("ns1", "key1").await.unwrap().unwrap();
        assert_eq!(item.value, json!({"data": 2}));

        // Search
        store.put("ns1", "key2", json!({"data": 3})).await.unwrap();
        let results = store.search("ns1", None, 10).await.unwrap();
        assert_eq!(results.len(), 2);

        // Search with query
        let results = store.search("ns1", Some("key1"), 10).await.unwrap();
        assert_eq!(results.len(), 1);

        // List namespaces
        store.put("ns2", "key1", json!(1)).await.unwrap();
        let namespaces = store.list_namespaces().await.unwrap();
        assert_eq!(namespaces.len(), 2);

        // Delete
        store.delete("ns1", "key1").await.unwrap();
        let item = store.get("ns1", "key1").await.unwrap();
        assert!(item.is_none());

        // Get from non-existent namespace
        let item = store.get("no_such_ns", "key1").await.unwrap();
        assert!(item.is_none());
    }
}

//! Base store trait for key-value storage.

use chrono::{DateTime, Utc};
use synwire_core::BoxFuture;

use crate::types::CheckpointError;

/// Trait for key-value storage with namespace isolation.
///
/// Used alongside `BaseCheckpointSaver` to store auxiliary data
/// (e.g., tool outputs, intermediate results) that are keyed by
/// namespace and key rather than by checkpoint ID.
pub trait BaseStore: Send + Sync {
    /// Retrieve a single item by namespace and key.
    fn get<'a>(
        &'a self,
        namespace: &'a str,
        key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Item>, CheckpointError>>;

    /// Insert or update an item.
    fn put<'a>(
        &'a self,
        namespace: &'a str,
        key: &'a str,
        value: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), CheckpointError>>;

    /// Delete an item by namespace and key.
    fn delete<'a>(
        &'a self,
        namespace: &'a str,
        key: &'a str,
    ) -> BoxFuture<'a, Result<(), CheckpointError>>;

    /// Search items within a namespace, optionally filtering by query string.
    fn search<'a>(
        &'a self,
        namespace: &'a str,
        query: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<SearchItem>, CheckpointError>>;

    /// List all namespaces that contain items.
    fn list_namespaces(&self) -> BoxFuture<'_, Result<Vec<String>, CheckpointError>>;
}

/// A stored item with metadata.
#[derive(Debug, Clone)]
pub struct Item {
    /// The namespace this item belongs to.
    pub namespace: String,
    /// The key within the namespace.
    pub key: String,
    /// The stored value.
    pub value: serde_json::Value,
    /// When this item was first created.
    pub created_at: DateTime<Utc>,
    /// When this item was last updated.
    pub updated_at: DateTime<Utc>,
}

/// A search result item with a relevance score.
#[derive(Debug, Clone)]
pub struct SearchItem {
    /// The matched item.
    pub item: Item,
    /// Relevance score (higher is more relevant).
    pub score: f32,
}

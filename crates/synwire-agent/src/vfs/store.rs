//! Persistent cross-conversation key-value store VFS provider.

use std::collections::BTreeMap;
use std::sync::RwLock;

use synwire_core::BoxFuture;
use synwire_core::vfs::error::VfsError;
use synwire_core::vfs::grep_options::GrepOptions;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{
    CpOptions, DirEntry, EditResult, FileContent, GlobEntry, GrepMatch, LsOptions, RmOptions,
    TransferResult, VfsCapabilities, WriteResult,
};

/// Namespaced key-value store that delegates to a [`BaseStore`].
///
/// In production this wraps a `SQLite` checkpoint.  In tests it wraps an
/// in-memory map.  All keys are namespaced by `namespace/key`.
pub trait BaseStore: Send + Sync {
    /// Read a value.
    fn get(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, VfsError>;
    /// Write a value.
    fn set(&self, namespace: &str, key: &str, value: Vec<u8>) -> Result<(), VfsError>;
    /// Delete a value.
    fn delete(&self, namespace: &str, key: &str) -> Result<(), VfsError>;
    /// List all keys in a namespace.
    fn list(&self, namespace: &str) -> Result<Vec<String>, VfsError>;
}

/// In-memory [`BaseStore`] implementation for tests.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    data: RwLock<BTreeMap<String, Vec<u8>>>,
}

impl InMemoryStore {
    /// Create a new empty in-memory store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn full_key(namespace: &str, key: &str) -> String {
        format!("{namespace}/{key}")
    }
}

impl BaseStore for InMemoryStore {
    fn get(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, VfsError> {
        let data = self
            .data
            .read()
            .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
        Ok(data.get(&Self::full_key(namespace, key)).cloned())
    }

    fn set(&self, namespace: &str, key: &str, value: Vec<u8>) -> Result<(), VfsError> {
        let _ = self
            .data
            .write()
            .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
            .insert(Self::full_key(namespace, key), value);
        Ok(())
    }

    fn delete(&self, namespace: &str, key: &str) -> Result<(), VfsError> {
        let removed = self
            .data
            .write()
            .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
            .remove(&Self::full_key(namespace, key));
        if removed.is_none() {
            return Err(VfsError::NotFound(key.to_string()));
        }
        Ok(())
    }

    fn list(&self, namespace: &str) -> Result<Vec<String>, VfsError> {
        let prefix = format!("{namespace}/");
        let keys = self
            .data
            .read()
            .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|k| k[prefix.len()..].to_string())
            .collect();
        Ok(keys)
    }
}

/// Backend that wraps a [`BaseStore`] and exposes it as a [`Vfs`].
///
/// Keys map to paths: `/<namespace>/<key>`.
pub struct StoreProvider {
    namespace: String,
    store: Box<dyn BaseStore>,
}

impl std::fmt::Debug for StoreProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreProvider")
            .field("namespace", &self.namespace)
            .finish_non_exhaustive()
    }
}

impl StoreProvider {
    /// Create a new store backend.
    pub fn new(namespace: impl Into<String>, store: impl BaseStore + 'static) -> Self {
        Self {
            namespace: namespace.into(),
            store: Box::new(store),
        }
    }
}

impl Vfs for StoreProvider {
    fn ls(&self, _path: &str, _opts: LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>> {
        Box::pin(async move {
            let keys = self.store.list(&self.namespace)?;
            let entries = keys
                .into_iter()
                .map(|k| DirEntry {
                    path: format!("/{}/{}", self.namespace, k),
                    name: k,
                    is_dir: false,
                    size: None,
                    modified: None,
                    permissions: None,
                    is_symlink: false,
                })
                .collect();
            Ok(entries)
        })
    }

    fn read(&self, path: &str) -> BoxFuture<'_, Result<FileContent, VfsError>> {
        let key = strip_namespace(path, &self.namespace);
        Box::pin(async move {
            let content = self
                .store
                .get(&self.namespace, &key)?
                .ok_or(VfsError::NotFound(key))?;
            Ok(FileContent {
                content,
                mime_type: None,
            })
        })
    }

    fn write(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<WriteResult, VfsError>> {
        let key = strip_namespace(path, &self.namespace);
        let content = content.to_vec();
        Box::pin(async move {
            let bytes = content.len() as u64;
            self.store.set(&self.namespace, &key, content)?;
            Ok(WriteResult {
                path: format!("/{}/{}", self.namespace, key),
                bytes_written: bytes,
            })
        })
    }

    fn edit(
        &self,
        path: &str,
        old: &str,
        new: &str,
    ) -> BoxFuture<'_, Result<EditResult, VfsError>> {
        let key = strip_namespace(path, &self.namespace);
        let old = old.to_string();
        let new = new.to_string();
        Box::pin(async move {
            let bytes = self
                .store
                .get(&self.namespace, &key)?
                .ok_or_else(|| VfsError::NotFound(key.clone()))?;
            let text = String::from_utf8(bytes)
                .map_err(|_| VfsError::Unsupported("binary content".into()))?;
            if !text.contains(&old) {
                return Ok(EditResult {
                    path: key,
                    edits_applied: 0,
                    content_after: Some(text),
                });
            }
            let replaced = text.replacen(&old, &new, 1);
            let after = replaced.clone();
            self.store
                .set(&self.namespace, &key, replaced.into_bytes())?;
            Ok(EditResult {
                path: key,
                edits_applied: 1,
                content_after: Some(after),
            })
        })
    }

    fn grep(
        &self,
        _pattern: &str,
        _opts: GrepOptions,
    ) -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "grep not supported on StoreProvider".into(),
            ))
        })
    }

    fn glob(&self, _pattern: &str) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "glob not supported on StoreProvider".into(),
            ))
        })
    }

    fn upload(&self, _from: &str, _to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "upload not supported on StoreProvider".into(),
            ))
        })
    }

    fn download(&self, _from: &str, _to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "download not supported on StoreProvider".into(),
            ))
        })
    }

    fn pwd(&self) -> BoxFuture<'_, Result<String, VfsError>> {
        let ns = self.namespace.clone();
        Box::pin(async move { Ok(format!("/{ns}")) })
    }

    fn cd(&self, _path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "cd not supported on StoreProvider".into(),
            ))
        })
    }

    fn rm(&self, path: &str, _opts: RmOptions) -> BoxFuture<'_, Result<(), VfsError>> {
        let key = strip_namespace(path, &self.namespace);
        Box::pin(async move { self.store.delete(&self.namespace, &key) })
    }

    fn cp(
        &self,
        _from: &str,
        _to: &str,
        _opts: CpOptions,
    ) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "cp not supported on StoreProvider".into(),
            ))
        })
    }

    fn mv_file(&self, _from: &str, _to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "mv not supported on StoreProvider".into(),
            ))
        })
    }

    fn capabilities(&self) -> VfsCapabilities {
        VfsCapabilities::READ | VfsCapabilities::WRITE | VfsCapabilities::RM
    }

    fn provider_name(&self) -> &'static str {
        "StoreProvider"
    }
}

fn strip_namespace(path: &str, namespace: &str) -> String {
    let prefix = format!("/{namespace}/");
    path.strip_prefix(&prefix)
        .unwrap_or_else(|| path.trim_start_matches('/'))
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cross_conversation_persistence() {
        let store = InMemoryStore::new();
        let backend = StoreProvider::new("agent1", store);

        let _ = backend
            .write("/agent1/key1", b"value1")
            .await
            .expect("write");

        let content = backend.read("/agent1/key1").await.expect("read");
        assert_eq!(content.content, b"value1");
    }

    #[tokio::test]
    async fn test_namespace_isolation() {
        use std::sync::Arc;

        struct SharedStore(Arc<InMemoryStore>);
        impl BaseStore for SharedStore {
            fn get(&self, ns: &str, key: &str) -> Result<Option<Vec<u8>>, VfsError> {
                self.0.get(ns, key)
            }
            fn set(&self, ns: &str, key: &str, val: Vec<u8>) -> Result<(), VfsError> {
                self.0.set(ns, key, val)
            }
            fn delete(&self, ns: &str, key: &str) -> Result<(), VfsError> {
                self.0.delete(ns, key)
            }
            fn list(&self, ns: &str) -> Result<Vec<String>, VfsError> {
                self.0.list(ns)
            }
        }

        let store = Arc::new(InMemoryStore::new());
        let b1 = StoreProvider::new("ns1", SharedStore(store.clone()));
        let b2 = StoreProvider::new("ns2", SharedStore(store.clone()));

        let _ = b1.write("/ns1/k", b"from-ns1").await.expect("write");
        // ns2 cannot see ns1's key.
        let err = b2.read("/ns2/k").await;
        assert!(err.is_err());
    }
}

//! Composite VFS provider routing by path prefix.

use synwire_core::BoxFuture;
use synwire_core::vfs::error::VfsError;
use synwire_core::vfs::grep_options::GrepOptions;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{
    CpOptions, DirEntry, EditResult, FileContent, GlobEntry, GrepMatch, LsOptions, MountInfo,
    RmOptions, TransferResult, VfsCapabilities, WriteResult,
};

/// A single mount point mapping a path prefix to a backend.
pub struct Mount {
    /// Prefix (e.g. `/store` or `/git`).  Must start with `/`.
    pub prefix: String,
    /// Provider serving this mount point.
    pub backend: Box<dyn Vfs>,
}

impl std::fmt::Debug for Mount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mount")
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

/// Routes operations to the provider whose prefix is the longest match.
///
/// Mounts are sorted by descending prefix length so the most specific
/// mount wins.  Segment-boundary matching is enforced: `/store` matches
/// `/store/foo` but not `/storefront`.
pub struct CompositeProvider {
    mounts: Vec<Mount>,
}

impl std::fmt::Debug for CompositeProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeProvider")
            .field(
                "mounts",
                &self.mounts.iter().map(|m| &m.prefix).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl CompositeProvider {
    /// Create a new composite backend from a list of mounts.
    ///
    /// Mounts are sorted by descending prefix length automatically.
    #[must_use]
    pub fn new(mut mounts: Vec<Mount>) -> Self {
        mounts.sort_by(|a, b| b.prefix.len().cmp(&a.prefix.len()));
        Self { mounts }
    }

    fn find_mount(&self, path: &str) -> Option<(&dyn Vfs, String)> {
        for mount in &self.mounts {
            let prefix = &mount.prefix;
            // Segment-boundary check: path must start with prefix followed by `/` or be equal.
            if path == prefix || path.starts_with(&format!("{}/", prefix.trim_end_matches('/'))) {
                // Strip the prefix to get the relative path for the backend.
                let stripped = path
                    .strip_prefix(prefix.trim_end_matches('/'))
                    .unwrap_or(path);
                let relative = if stripped.is_empty() { "/" } else { stripped };
                return Some((mount.backend.as_ref(), relative.to_string()));
            }
        }
        None
    }
}

macro_rules! delegate {
    ($self:expr, $path:expr, $method:ident $(, $arg:expr)*) => {{
        let path = $path.to_string();
        Box::pin(async move {
            match $self.find_mount(&path) {
                Some((backend, relative)) => backend.$method(&relative, $($arg,)*).await,
                None => Err(VfsError::NotFound(path)),
            }
        })
    }};
}

impl Vfs for CompositeProvider {
    fn ls(&self, path: &str, opts: LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>> {
        let path_str = path.to_string();
        Box::pin(async move {
            if let Some((backend, relative)) = self.find_mount(&path_str) {
                return backend.ls(&relative, opts).await;
            }
            // Root or unmatched path: show mount points as directories
            if path_str == "/" || path_str.is_empty() || path_str == "." {
                let entries = self
                    .mounts
                    .iter()
                    .map(|m| {
                        let name = m
                            .prefix
                            .trim_start_matches('/')
                            .split('/')
                            .next()
                            .unwrap_or(&m.prefix);
                        DirEntry {
                            name: name.to_string(),
                            path: m.prefix.clone(),
                            is_dir: true,
                            size: None,
                            modified: None,
                            permissions: None,
                            is_symlink: false,
                        }
                    })
                    .collect();
                return Ok(entries);
            }
            Err(VfsError::NotFound(path_str))
        })
    }

    fn read(&self, path: &str) -> BoxFuture<'_, Result<FileContent, VfsError>> {
        delegate!(self, path, read)
    }

    fn write(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<WriteResult, VfsError>> {
        let path = path.to_string();
        let content = content.to_vec();
        Box::pin(async move {
            match self.find_mount(&path) {
                Some((backend, relative)) => backend.write(&relative, &content).await,
                None => Err(VfsError::NotFound(path)),
            }
        })
    }

    fn edit(
        &self,
        path: &str,
        old: &str,
        new: &str,
    ) -> BoxFuture<'_, Result<EditResult, VfsError>> {
        let path = path.to_string();
        let old = old.to_string();
        let new = new.to_string();
        Box::pin(async move {
            match self.find_mount(&path) {
                Some((backend, relative)) => backend.edit(&relative, &old, &new).await,
                None => Err(VfsError::NotFound(path)),
            }
        })
    }

    fn grep(
        &self,
        pattern: &str,
        opts: GrepOptions,
    ) -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>> {
        let pattern = pattern.to_string();
        Box::pin(async move {
            let mut all = Vec::new();
            for mount in &self.mounts {
                if mount.backend.capabilities().contains(VfsCapabilities::GREP) {
                    if let Ok(mut matches) = mount.backend.grep(&pattern, opts.clone()).await {
                        // Prefix match file paths with mount prefix
                        for m in &mut matches {
                            if !m.file.starts_with(&mount.prefix) {
                                let suffix = if m.file.starts_with('/') {
                                    m.file.clone()
                                } else {
                                    format!("/{}", m.file)
                                };
                                m.file =
                                    format!("{}{}", mount.prefix.trim_end_matches('/'), suffix,);
                            }
                        }
                        all.append(&mut matches);
                    }
                }
            }
            if all.is_empty()
                && !self
                    .mounts
                    .iter()
                    .any(|m| m.backend.capabilities().contains(VfsCapabilities::GREP))
            {
                return Err(VfsError::Unsupported(
                    "no mounted provider supports grep".into(),
                ));
            }
            Ok(all)
        })
    }

    fn glob(&self, pattern: &str) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>> {
        // Aggregate from all mounts that support GLOB.
        let pattern = pattern.to_string();
        Box::pin(async move {
            let mut all = Vec::new();
            for mount in &self.mounts {
                if mount.backend.capabilities().contains(VfsCapabilities::GLOB) {
                    let mut entries = mount.backend.glob(&pattern).await?;
                    all.append(&mut entries);
                }
            }
            Ok(all)
        })
    }

    fn upload(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let to = to.to_string();
        let from = from.to_string();
        Box::pin(async move {
            match self.find_mount(&to) {
                Some((backend, relative)) => backend.upload(&from, &relative).await,
                None => Err(VfsError::NotFound(to)),
            }
        })
    }

    fn download(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            match self.find_mount(&from) {
                Some((backend, relative)) => backend.download(&relative, &to).await,
                None => Err(VfsError::NotFound(from)),
            }
        })
    }

    fn pwd(&self) -> BoxFuture<'_, Result<String, VfsError>> {
        Box::pin(async { Ok("/".to_string()) })
    }

    fn cd(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        delegate!(self, path, cd)
    }

    fn rm(&self, path: &str, opts: RmOptions) -> BoxFuture<'_, Result<(), VfsError>> {
        delegate!(self, path, rm, opts)
    }

    fn cp(
        &self,
        from: &str,
        to: &str,
        opts: CpOptions,
    ) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            let src_mount = self.find_mount(&from);
            let dst_mount = self.find_mount(&to);
            match (src_mount, dst_mount) {
                (Some((src_backend, src_rel)), Some((dst_backend, dst_rel))) => {
                    if std::ptr::eq(src_backend, dst_backend) {
                        return src_backend.cp(&src_rel, &dst_rel, opts).await;
                    }
                    // Cross-boundary: read from source, write to destination
                    if opts.no_overwrite && dst_backend.stat(&dst_rel).await.is_ok() {
                        return Ok(TransferResult {
                            path: to,
                            bytes_transferred: 0,
                        });
                    }
                    let content = src_backend.read(&src_rel).await?;
                    let result = dst_backend.write(&dst_rel, &content.content).await?;
                    Ok(TransferResult {
                        path: to,
                        bytes_transferred: result.bytes_written,
                    })
                }
                (None, _) => Err(VfsError::NotFound(from)),
                (_, None) => Err(VfsError::NotFound(to)),
            }
        })
    }

    fn mv_file(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            let src_mount = self.find_mount(&from);
            let dst_mount = self.find_mount(&to);
            match (src_mount, dst_mount) {
                (Some((src_backend, src_rel)), Some((dst_backend, dst_rel))) => {
                    if std::ptr::eq(src_backend, dst_backend) {
                        return src_backend.mv_file(&src_rel, &dst_rel).await;
                    }
                    // Cross-boundary: read, write, delete
                    let content = src_backend.read(&src_rel).await?;
                    let bytes = content.content.len() as u64;
                    let _ = dst_backend.write(&dst_rel, &content.content).await?;
                    src_backend.rm(&src_rel, RmOptions::default()).await?;
                    Ok(TransferResult {
                        path: to,
                        bytes_transferred: bytes,
                    })
                }
                (None, _) => Err(VfsError::NotFound(from)),
                (_, None) => Err(VfsError::NotFound(to)),
            }
        })
    }

    fn capabilities(&self) -> VfsCapabilities {
        self.mounts.iter().fold(VfsCapabilities::empty(), |acc, m| {
            acc | m.backend.capabilities()
        })
    }

    fn provider_name(&self) -> &'static str {
        "CompositeProvider"
    }

    fn mount_info(&self) -> Vec<MountInfo> {
        self.mounts
            .iter()
            .map(|m| {
                let caps = m.backend.capabilities();
                MountInfo {
                    prefix: m.prefix.clone(),
                    provider: m.backend.provider_name().to_string(),
                    capabilities: synwire_core::vfs::protocol::capability_names(caps),
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use synwire_core::vfs::MemoryProvider;

    fn make_composite() -> CompositeProvider {
        let fs1 = Box::new(MemoryProvider::new());
        let fs2 = Box::new(MemoryProvider::new());
        CompositeProvider::new(vec![
            Mount {
                prefix: "/store".to_string(),
                backend: fs1,
            },
            Mount {
                prefix: "/git".to_string(),
                backend: fs2,
            },
        ])
    }

    #[tokio::test]
    async fn test_composite_routing() {
        let composite = make_composite();
        let _ = composite
            .write("/store/key1", b"data")
            .await
            .expect("write to /store");

        let content = composite.read("/store/key1").await.expect("read /store");
        assert_eq!(content.content, b"data");
    }

    #[tokio::test]
    async fn test_path_traversal_rejection() {
        let composite = make_composite();
        // /storefront must NOT match /store.
        let err = composite.write("/storefront/f", b"x").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_longer_prefix_wins() {
        let deep = Box::new(MemoryProvider::new());
        let shallow = Box::new(MemoryProvider::new());
        let composite = CompositeProvider::new(vec![
            Mount {
                prefix: "/a/b".to_string(),
                backend: deep,
            },
            Mount {
                prefix: "/a".to_string(),
                backend: shallow,
            },
        ]);
        // /a/b/file should go to the /a/b mount.
        let _ = composite.write("/a/b/file", b"deep").await.expect("write");
        // /a/other should go to the /a mount.
        let _ = composite
            .write("/a/other", b"shallow")
            .await
            .expect("write shallow");
    }
}

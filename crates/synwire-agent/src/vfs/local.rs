//! Local filesystem VFS provider.

use std::collections::HashMap;
use std::path::{Component, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;

use synwire_core::BoxFuture;
use synwire_core::vfs::agentic_ignore::AgenticIgnore;
use synwire_core::vfs::error::VfsError;
use synwire_core::vfs::grep_options::{GrepOptions, GrepOutputMode};
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{
    CpOptions, DirEntry, EditResult, FileContent, FindEntry, FindOptions, FindType, GlobEntry,
    GrepMatch, LsOptions, RmOptions, TransferResult, VfsCapabilities, WriteResult,
};

use regex::Regex;

#[cfg(feature = "semantic-search")]
use {
    once_cell::sync::OnceCell as OnceLock,
    std::path::Path,
    synwire_core::vectorstores::VectorStore,
    synwire_core::vfs::types::{
        IndexHandle, IndexOptions, IndexStatus, SemanticSearchOptions, SemanticSearchResult,
    },
    synwire_embeddings_local::{LocalEmbeddings, LocalReranker},
    synwire_index::{IndexConfig, SemanticIndex, StoreFactory},
    synwire_vectorstore_lancedb::LanceDbVectorStore,
};

/// Local filesystem VFS provider with path-traversal protection.
///
/// All operations are scoped to `root`.  Relative paths are resolved from the
/// current working directory (`cwd`), which itself must stay inside `root`.
pub struct LocalProvider {
    root: PathBuf,
    cwd: Mutex<PathBuf>,
    watched: Arc<RwLock<HashMap<String, SystemTime>>>,
    agentic_ignore: AgenticIgnore,
    #[cfg(feature = "semantic-search")]
    semantic_index: OnceLock<Arc<SemanticIndex>>,
}

impl std::fmt::Debug for LocalProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalProvider")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalProvider {
    /// Create a new provider rooted at `root`.
    ///
    /// # Errors
    ///
    /// Returns an error if `root` does not exist or is not a directory.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, VfsError> {
        let root = root.into().canonicalize()?;
        if !root.is_dir() {
            return Err(VfsError::NotFound(root.display().to_string()));
        }
        let cwd = root.clone();
        let agentic_ignore = AgenticIgnore::discover(&root);
        Ok(Self {
            root,
            cwd: Mutex::new(cwd),
            watched: Arc::new(RwLock::new(HashMap::new())),
            agentic_ignore,
            #[cfg(feature = "semantic-search")]
            semantic_index: OnceLock::new(),
        })
    }

    /// Lazily initialise and return the shared [`SemanticIndex`].
    ///
    /// On first call, creates `LocalEmbeddings`, `LocalReranker`, and a
    /// LanceDB-backed store factory.  Subsequent calls return the cached value.
    /// Lazily initialise and return the shared [`SemanticIndex`].
    ///
    /// On first call, creates `LocalEmbeddings`, `LocalReranker`, and a
    /// LanceDB-backed store factory.  Subsequent calls return the cached value.
    #[cfg(feature = "semantic-search")]
    fn get_or_init_index(&self) -> Result<&Arc<SemanticIndex>, VfsError> {
        // OnceLock::get_or_try_init is stable since Rust 1.83 (our MSRV is 1.85).
        self.semantic_index.get_or_try_init(|| {
            let embeddings = LocalEmbeddings::new()
                .map_err(|e| VfsError::Io(std::io::Error::other(e.to_string())))?;
            let reranker = LocalReranker::new()
                .map_err(|e| VfsError::Io(std::io::Error::other(e.to_string())))?;
            let dims = 384usize;
            let factory: StoreFactory = Box::new(move |cache_dir: &Path| {
                let lance_path = cache_dir.join("lance");
                let path_str = lance_path.to_string_lossy().to_string();
                // LanceDbVectorStore::open is async; run it on the current
                // tokio handle via block_in_place (requires multi-thread runtime).
                let store = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(LanceDbVectorStore::open(&path_str, "chunks", dims))
                })
                .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;
                Ok(Arc::new(store) as Arc<dyn VectorStore>)
            });
            let idx = SemanticIndex::new(
                Arc::new(embeddings),
                Some(Arc::new(reranker)),
                factory,
                IndexConfig::default(),
                None,
            );
            Ok(Arc::new(idx))
        })
    }

    /// Resolve `path` relative to cwd, rejecting traversal outside root.
    fn resolve(&self, path: &str) -> Result<PathBuf, VfsError> {
        let cwd = self
            .cwd
            .lock()
            .map_err(|_| VfsError::Unsupported("mutex poisoned".into()))?
            .clone();

        let candidate = if path.starts_with('/') {
            self.root.join(path.trim_start_matches('/'))
        } else {
            cwd.join(path)
        };

        // Canonicalise without requiring the path to exist.
        let normalised = normalise_path(&candidate);

        if !normalised.starts_with(&self.root) {
            return Err(VfsError::PathTraversal {
                attempted: normalised.display().to_string(),
                root: self.root.display().to_string(),
            });
        }
        Ok(normalised)
    }
}

/// Normalise a path (collapse `.` / `..`) without requiring existence.
fn normalise_path(path: &std::path::Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(std::path::MAIN_SEPARATOR_STR),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::Normal(n) => out.push(n),
        }
    }
    out
}

impl Vfs for LocalProvider {
    fn ls(&self, path: &str, _opts: LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            let mut entries = Vec::new();
            let mut rd = tokio::fs::read_dir(&resolved).await.map_err(VfsError::Io)?;
            while let Some(entry) = rd.next_entry().await.map_err(VfsError::Io)? {
                if self
                    .agentic_ignore
                    .is_ignored(&entry.path(), entry.path().is_dir())
                {
                    continue;
                }
                let meta = entry.metadata().await.map_err(VfsError::Io)?;
                #[cfg(unix)]
                let permissions = {
                    use std::os::unix::fs::PermissionsExt;
                    Some(meta.permissions().mode())
                };
                #[cfg(not(unix))]
                let permissions: Option<u32> = None;

                entries.push(DirEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path().display().to_string(),
                    is_dir: meta.is_dir(),
                    size: if meta.is_file() {
                        Some(meta.len())
                    } else {
                        None
                    },
                    modified: meta.modified().ok().and_then(|t| {
                        let secs = t
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        chrono::DateTime::from_timestamp(i64::try_from(secs).unwrap_or(i64::MAX), 0)
                    }),
                    permissions,
                    is_symlink: meta.is_symlink(),
                });
            }
            Ok(entries)
        })
    }

    fn read(&self, path: &str) -> BoxFuture<'_, Result<FileContent, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            let content = tokio::fs::read(&resolved).await.map_err(VfsError::Io)?;
            Ok(FileContent {
                content,
                mime_type: None,
            })
        })
    }

    fn write(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<WriteResult, VfsError>> {
        let path = path.to_string();
        let content = content.to_vec();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            if let Some(parent) = resolved.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(VfsError::Io)?;
            }
            let bytes_written = content.len() as u64;
            tokio::fs::write(&resolved, &content)
                .await
                .map_err(VfsError::Io)?;
            Ok(WriteResult {
                path: resolved.display().to_string(),
                bytes_written,
            })
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
            let resolved = self.resolve(&path)?;
            let bytes = tokio::fs::read(&resolved).await.map_err(VfsError::Io)?;
            let text = String::from_utf8(bytes)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;
            if !text.contains(&old) {
                return Ok(EditResult {
                    path: resolved.display().to_string(),
                    edits_applied: 0,
                    content_after: Some(text),
                });
            }
            let replaced = text.replacen(&old, &new, 1);
            let after = replaced.clone();
            tokio::fs::write(&resolved, replaced.as_bytes())
                .await
                .map_err(VfsError::Io)?;
            Ok(EditResult {
                path: resolved.display().to_string(),
                edits_applied: 1,
                content_after: Some(after),
            })
        })
    }

    fn grep(
        &self,
        pattern: &str,
        opts: GrepOptions,
    ) -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>> {
        let pattern = pattern.to_string();
        Box::pin(async move {
            let regex_pattern = if opts.case_insensitive {
                format!("(?i){pattern}")
            } else {
                pattern
            };
            let re = Regex::new(&regex_pattern)
                .map_err(|e| VfsError::Unsupported(format!("invalid regex: {e}")))?;

            let root = match &opts.path {
                Some(p) => self.resolve(p)?,
                None => self
                    .cwd
                    .lock()
                    .map_err(|_| VfsError::Unsupported("mutex poisoned".into()))?
                    .clone(),
            };

            let after = opts.context.unwrap_or(opts.after_context);
            let before = opts.context.unwrap_or(opts.before_context);
            let mut matches = Vec::new();
            let mut total = 0usize;

            grep_dir(
                &root,
                &re,
                &opts,
                before,
                after,
                &mut matches,
                &mut total,
                &self.agentic_ignore,
            )?;
            Ok(matches)
        })
    }

    fn glob(&self, pattern: &str) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>> {
        let pattern = pattern.to_string();
        Box::pin(async move {
            let root = self
                .cwd
                .lock()
                .map_err(|_| VfsError::Unsupported("mutex poisoned".into()))?
                .clone();
            let mut entries = Vec::new();
            glob_dir(&root, &pattern, &mut entries, &self.agentic_ignore)?;
            Ok(entries)
        })
    }

    fn upload(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            let dst = self.resolve(&to)?;
            let content = tokio::fs::read(&from).await.map_err(VfsError::Io)?;
            let bytes = content.len() as u64;
            tokio::fs::write(&dst, &content)
                .await
                .map_err(VfsError::Io)?;
            Ok(TransferResult {
                path: dst.display().to_string(),
                bytes_transferred: bytes,
            })
        })
    }

    fn download(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            let src = self.resolve(&from)?;
            let content = tokio::fs::read(&src).await.map_err(VfsError::Io)?;
            let bytes = content.len() as u64;
            tokio::fs::write(&to, &content)
                .await
                .map_err(VfsError::Io)?;
            Ok(TransferResult {
                path: to,
                bytes_transferred: bytes,
            })
        })
    }

    fn pwd(&self) -> BoxFuture<'_, Result<String, VfsError>> {
        Box::pin(async move {
            self.cwd
                .lock()
                .map(|g| g.display().to_string())
                .map_err(|_| VfsError::Unsupported("mutex poisoned".into()))
        })
    }

    fn cd(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            if !resolved.is_dir() {
                return Err(VfsError::NotFound(resolved.display().to_string()));
            }
            *self
                .cwd
                .lock()
                .map_err(|_| VfsError::Unsupported("mutex poisoned".into()))? = resolved;
            Ok(())
        })
    }

    fn rm(&self, path: &str, _opts: RmOptions) -> BoxFuture<'_, Result<(), VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            if resolved.is_dir() {
                tokio::fs::remove_dir_all(&resolved)
                    .await
                    .map_err(VfsError::Io)?;
            } else {
                tokio::fs::remove_file(&resolved)
                    .await
                    .map_err(VfsError::Io)?;
            }
            Ok(())
        })
    }

    fn cp(
        &self,
        from: &str,
        to: &str,
        _opts: CpOptions,
    ) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            let src = self.resolve(&from)?;
            let dst = self.resolve(&to)?;
            let bytes = tokio::fs::copy(&src, &dst).await.map_err(VfsError::Io)?;
            Ok(TransferResult {
                path: dst.display().to_string(),
                bytes_transferred: bytes,
            })
        })
    }

    fn mv_file(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            let src = self.resolve(&from)?;
            let dst = self.resolve(&to)?;
            let meta = tokio::fs::metadata(&src).await.map_err(VfsError::Io)?;
            let bytes = meta.len();
            tokio::fs::rename(&src, &dst).await.map_err(VfsError::Io)?;
            Ok(TransferResult {
                path: dst.display().to_string(),
                bytes_transferred: bytes,
            })
        })
    }

    fn watch(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            let mtime = std::fs::metadata(&resolved)?.modified()?;
            let key = resolved.display().to_string();
            let _ = self
                .watched
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
                .insert(key, mtime);
            Ok(())
        })
    }

    fn check_stale(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            let key = resolved.display().to_string();
            let recorded = {
                let guard = self
                    .watched
                    .read()
                    .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
                match guard.get(&key) {
                    Some(&t) => t,
                    None => return Ok(()),
                }
            };
            let current = std::fs::metadata(&resolved)?.modified()?;
            if current != recorded {
                return Err(VfsError::StaleRead { path });
            }
            Ok(())
        })
    }

    fn find(
        &self,
        path: &str,
        opts: FindOptions,
    ) -> BoxFuture<'_, Result<Vec<FindEntry>, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            let mut results = Vec::new();
            find_dir(&resolved, &opts, 0, &mut results, &self.agentic_ignore)?;
            Ok(results)
        })
    }

    #[cfg(feature = "semantic-search")]
    fn index(
        &self,
        path: &str,
        opts: IndexOptions,
    ) -> BoxFuture<'_, Result<IndexHandle, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let resolved = self.resolve(&path)?;
            let idx = self.get_or_init_index()?;
            idx.index(&resolved, &opts).await
        })
    }

    #[cfg(feature = "semantic-search")]
    fn index_status(&self, index_id: &str) -> BoxFuture<'_, Result<IndexStatus, VfsError>> {
        let id = index_id.to_string();
        Box::pin(async move {
            let idx = self.get_or_init_index()?;
            idx.status(&id).await
        })
    }

    #[cfg(feature = "semantic-search")]
    fn semantic_search(
        &self,
        query: &str,
        opts: SemanticSearchOptions,
    ) -> BoxFuture<'_, Result<Vec<SemanticSearchResult>, VfsError>> {
        let query = query.to_string();
        let root = self.root.clone();
        Box::pin(async move {
            let idx = self.get_or_init_index()?;
            idx.search(&root, &query, &opts).await
        })
    }

    fn capabilities(&self) -> VfsCapabilities {
        #[cfg(feature = "semantic-search")]
        return VfsCapabilities::all();
        #[cfg(not(feature = "semantic-search"))]
        return VfsCapabilities::all()
            & !VfsCapabilities::INDEX
            & !VfsCapabilities::SEMANTIC_SEARCH;
    }

    fn provider_name(&self) -> &'static str {
        "LocalProvider"
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn grep_dir(
    dir: &std::path::Path,
    re: &Regex,
    opts: &GrepOptions,
    before: u32,
    after: u32,
    matches: &mut Vec<GrepMatch>,
    total: &mut usize,
    agentic_ignore: &AgenticIgnore,
) -> Result<(), VfsError> {
    let rd = std::fs::read_dir(dir)?;
    for entry in rd {
        let entry = entry?;
        let path = entry.path();
        if agentic_ignore.is_ignored(&path, path.is_dir()) {
            continue;
        }
        if path.is_dir() {
            grep_dir(
                &path,
                re,
                opts,
                before,
                after,
                matches,
                total,
                agentic_ignore,
            )?;
            continue;
        }
        if let Some(ft) = &opts.file_type {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !synwire_core::vfs::memory::matches_file_type_pub(ft, ext) {
                continue;
            }
        }
        if let Some(glob) = &opts.glob {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !synwire_core::vfs::memory::glob_matches_pub(glob, name) {
                continue;
            }
        }
        let Ok(content) = std::fs::read(&path) else {
            continue;
        };
        if content.contains(&0u8) {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&content) else {
            continue;
        };
        let file_str = path.display().to_string();
        let lines: Vec<&str> = text.lines().collect();
        let mut file_count = 0;
        let mode = opts.output_mode;

        for (i, &line) in lines.iter().enumerate() {
            let line_matches = if opts.invert {
                !re.is_match(line)
            } else {
                re.is_match(line)
            };
            if !line_matches {
                continue;
            }
            file_count += 1;
            *total += 1;
            if mode == GrepOutputMode::FilesWithMatches {
                matches.push(GrepMatch {
                    file: file_str.clone(),
                    line_number: 0,
                    column: 0,
                    line_content: String::new(),
                    before: Vec::new(),
                    after: Vec::new(),
                });
                break;
            }
            if mode == GrepOutputMode::Count {
                continue;
            }
            let b_start = i.saturating_sub(before as usize);
            let a_end = (i + 1 + after as usize).min(lines.len());
            matches.push(GrepMatch {
                file: file_str.clone(),
                line_number: if opts.line_numbers { i + 1 } else { 0 },
                column: if opts.invert {
                    0
                } else {
                    re.find(line).map_or(0, |m| m.start())
                },
                line_content: line.to_string(),
                before: lines[b_start..i].iter().map(ToString::to_string).collect(),
                after: lines[i + 1..a_end]
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            });
            if let Some(max) = opts.max_matches
                && *total >= max
            {
                return Ok(());
            }
        }
        if mode == GrepOutputMode::Count && file_count > 0 {
            matches.push(GrepMatch {
                file: file_str,
                line_number: file_count,
                column: 0,
                line_content: file_count.to_string(),
                before: Vec::new(),
                after: Vec::new(),
            });
        }
    }
    Ok(())
}

fn glob_dir(
    dir: &std::path::Path,
    pattern: &str,
    entries: &mut Vec<GlobEntry>,
    agentic_ignore: &AgenticIgnore,
) -> Result<(), VfsError> {
    let rd = std::fs::read_dir(dir)?;
    for entry in rd {
        let entry = entry?;
        let path = entry.path();
        if agentic_ignore.is_ignored(&path, path.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            glob_dir(&path, pattern, entries, agentic_ignore)?;
        }
        if synwire_core::vfs::memory::glob_matches_pub(pattern, &name) {
            let meta = entry.metadata().ok();
            entries.push(GlobEntry {
                path: path.display().to_string(),
                is_dir: path.is_dir(),
                size: meta
                    .as_ref()
                    .filter(|m| m.is_file())
                    .map(std::fs::Metadata::len),
            });
        }
    }
    Ok(())
}

fn find_dir(
    dir: &std::path::Path,
    opts: &FindOptions,
    depth: usize,
    results: &mut Vec<FindEntry>,
    agentic_ignore: &AgenticIgnore,
) -> Result<(), VfsError> {
    if let Some(max) = opts.max_depth
        && depth > max
    {
        return Ok(());
    }
    let rd = std::fs::read_dir(dir)?;
    for entry in rd {
        let entry = entry?;
        let path = entry.path();
        let is_dir = path.is_dir();
        let is_symlink = path.symlink_metadata().is_ok_and(|m| m.is_symlink());

        if agentic_ignore.is_ignored(&path, is_dir) {
            continue;
        }

        let meta = entry.metadata().ok();

        // Type filter — still recurse into directories even if they don't match.
        if let Some(ref ft) = opts.entry_type {
            let matches_type = match ft {
                FindType::File => !is_dir && !is_symlink,
                FindType::Directory => is_dir,
                FindType::Symlink => is_symlink,
                _ => true,
            };
            if !matches_type {
                if is_dir {
                    find_dir(&path, opts, depth + 1, results, agentic_ignore)?;
                }
                continue;
            }
        }

        // Name glob filter — still recurse into directories.
        if let Some(ref name_pat) = opts.name {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !synwire_core::vfs::memory::glob_matches_pub(name_pat, &name) {
                if is_dir {
                    find_dir(&path, opts, depth + 1, results, agentic_ignore)?;
                }
                continue;
            }
        }

        // Size filters (files only).
        if let Some(ref m) = meta
            && !is_dir
        {
            if let Some(min) = opts.min_size
                && m.len() < min
            {
                continue;
            }
            if let Some(max) = opts.max_size
                && m.len() > max
            {
                continue;
            }
        }

        // Time filters — still recurse into directories.
        let modified_dt = meta.as_ref().and_then(|m| {
            let secs = m
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            chrono::DateTime::from_timestamp(i64::try_from(secs).unwrap_or(i64::MAX), 0)
        });

        if let Some(ref newer) = opts.newer_than
            && modified_dt.is_none_or(|t| t < *newer)
        {
            if is_dir {
                find_dir(&path, opts, depth + 1, results, agentic_ignore)?;
            }
            continue;
        }
        if let Some(ref older) = opts.older_than
            && modified_dt.is_none_or(|t| t > *older)
        {
            if is_dir {
                find_dir(&path, opts, depth + 1, results, agentic_ignore)?;
            }
            continue;
        }

        results.push(FindEntry {
            path: path.display().to_string(),
            is_dir,
            is_symlink,
            size: meta
                .as_ref()
                .filter(|_| !is_dir)
                .map(std::fs::Metadata::len),
            modified: modified_dt,
        });

        if is_dir {
            find_dir(&path, opts, depth + 1, results, agentic_ignore)?;
        }
    }
    Ok(())
}

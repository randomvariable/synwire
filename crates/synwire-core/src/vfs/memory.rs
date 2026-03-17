//! Ephemeral in-memory VFS provider implementation.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::{Arc, RwLock};

use regex::Regex;

use crate::BoxFuture;
use crate::vfs::error::VfsError;
use crate::vfs::grep_options::{GrepOptions, GrepOutputMode};
use crate::vfs::protocol::Vfs;
use crate::vfs::types::{
    CpOptions, DiffHunk, DiffLine, DiffOptions, DiffResult, DirEntry, DiskUsage, DiskUsageEntry,
    DuOptions, EditResult, FileContent, FileInfo, FindEntry, FindOptions, FindType, GlobEntry,
    GrepMatch, HeadTailOptions, LsOptions, MkdirOptions, ReadRange, RmOptions, SortField,
    TransferResult, TreeEntry, TreeOptions, VfsCapabilities, WordCount, WriteResult,
};

/// Ephemeral in-memory VFS provider.
///
/// All data lives for the lifetime of the backend instance.
/// Suitable for agent scratchpads and test fixtures.
#[derive(Debug, Clone)]
pub struct MemoryProvider {
    files: Arc<RwLock<BTreeMap<String, Vec<u8>>>>,
    cwd: Arc<Mutex<String>>,
}

impl Default for MemoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProvider {
    /// Create a new empty provider with `/` as the working directory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(BTreeMap::new())),
            cwd: Arc::new(Mutex::new("/".to_string())),
        }
    }

    /// Resolve `path` relative to the current working directory.
    fn resolve(cwd: &str, path: &str) -> Result<String, VfsError> {
        let base = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", cwd.trim_end_matches('/'), path)
        };

        // Normalise (collapse . and ..) and reject traversal.
        let mut parts: Vec<&str> = Vec::new();
        for seg in base.split('/') {
            match seg {
                "" | "." => {}
                ".." => {
                    if parts.is_empty() {
                        return Err(VfsError::PathTraversal {
                            attempted: base.clone(),
                            root: "/".to_string(),
                        });
                    }
                    let _ = parts.pop();
                }
                s => parts.push(s),
            }
        }
        let mut out = String::from("/");
        out.push_str(&parts.join("/"));
        Ok(out)
    }

    fn current_cwd(&self) -> Result<String, VfsError> {
        self.cwd
            .lock()
            .map(|g| g.clone())
            .map_err(|_| VfsError::Unsupported("mutex poisoned".into()))
    }
}

impl Vfs for MemoryProvider {
    fn ls(&self, path: &str, opts: LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let prefix = if resolved == "/" {
                "/".to_string()
            } else {
                format!("{}/", resolved.trim_end_matches('/'))
            };

            let files = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;

            let mut seen = std::collections::HashSet::new();
            let mut entries = Vec::new();

            for key in files.keys() {
                if !key.starts_with(&prefix) {
                    continue;
                }
                let rest = &key[prefix.len()..];
                let component = if opts.recursive {
                    rest
                } else {
                    rest.split('/').next().unwrap_or("")
                };
                if component.is_empty() {
                    continue;
                }
                // Skip hidden files unless -a.
                if !opts.all && component.starts_with('.') {
                    continue;
                }
                let is_dir = !opts.recursive && rest.contains('/');
                let entry_name = component.to_string();
                let entry_path = format!("{prefix}{entry_name}");
                if seen.insert(entry_path.clone()) {
                    let size = if is_dir {
                        None
                    } else {
                        files.get(key).map(|v| v.len() as u64)
                    };
                    entries.push(DirEntry {
                        name: entry_name,
                        path: entry_path,
                        is_dir,
                        size,
                        modified: None,
                        permissions: None,
                        is_symlink: false,
                    });
                }
            }
            drop(files);

            // Sort.
            match opts.sort {
                SortField::Name => entries.sort_by(|a, b| a.name.cmp(&b.name)),
                SortField::Size => entries.sort_by(|a, b| a.size.cmp(&b.size)),
                SortField::Time => entries.sort_by(|a, b| a.modified.cmp(&b.modified)),
                SortField::None => {}
            }
            if opts.reverse {
                entries.reverse();
            }

            Ok(entries)
        })
    }

    fn read(&self, path: &str) -> BoxFuture<'_, Result<FileContent, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let content = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
                .get(&resolved)
                .cloned()
                .ok_or_else(|| VfsError::NotFound(resolved.clone()))?;
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
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let bytes_written = content.len() as u64;
            let _ = self
                .files
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
                .insert(resolved.clone(), content);
            Ok(WriteResult {
                path: resolved,
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
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let bytes = {
                let files = self
                    .files
                    .read()
                    .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
                files
                    .get(&resolved)
                    .cloned()
                    .ok_or_else(|| VfsError::NotFound(resolved.clone()))?
            };
            let text = String::from_utf8(bytes)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;
            if !text.contains(&old) {
                return Ok(EditResult {
                    path: resolved,
                    edits_applied: 0,
                    content_after: Some(text),
                });
            }
            let replaced = text.replacen(&old, &new, 1);
            let content_after = replaced.clone();
            let _ = self
                .files
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
                .insert(resolved.clone(), replaced.into_bytes());
            Ok(EditResult {
                path: resolved,
                edits_applied: 1,
                content_after: Some(content_after),
            })
        })
    }

    #[allow(clippy::too_many_lines, clippy::significant_drop_tightening)]
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

            let files = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;

            let after = opts.context.unwrap_or(opts.after_context);
            let before = opts.context.unwrap_or(opts.before_context);
            let mut matches: Vec<GrepMatch> = Vec::new();
            let mut total = 0usize;

            // Determine search root.
            let cwd = self.current_cwd()?;
            let search_root = match &opts.path {
                Some(p) => Self::resolve(&cwd, p)?,
                None => cwd,
            };
            let prefix = if search_root == "/" {
                "/".to_string()
            } else {
                format!("{}/", search_root.trim_end_matches('/'))
            };

            'file_loop: for (file_path, content) in files.iter() {
                // Restrict to search root.
                if !file_path.starts_with(&prefix) && file_path != &search_root {
                    continue;
                }

                // File type filter.
                if let Some(ft) = &opts.file_type {
                    let ext = file_path.rsplit('.').next().unwrap_or("");
                    if !matches_file_type(ft, ext) {
                        continue;
                    }
                }

                // Glob filter.
                if let Some(glob) = &opts.glob {
                    let name = file_path.rsplit('/').next().unwrap_or("");
                    if !glob_matches(glob, name) {
                        continue;
                    }
                }

                // Skip binary (contains null byte).
                if content.contains(&0u8) {
                    continue;
                }

                let Ok(text) = std::str::from_utf8(content) else {
                    continue;
                };

                let lines: Vec<&str> = text.lines().collect();
                let mut file_match_count = 0usize;

                for (line_idx, &line) in lines.iter().enumerate() {
                    let is_matched = if opts.invert {
                        !re.is_match(line)
                    } else {
                        re.is_match(line)
                    };

                    if !is_matched {
                        continue;
                    }

                    file_match_count += 1;
                    total += 1;

                    if opts.output_mode == GrepOutputMode::FilesWithMatches {
                        matches.push(GrepMatch {
                            file: file_path.clone(),
                            line_number: 0,
                            column: 0,
                            line_content: String::new(),
                            before: Vec::new(),
                            after: Vec::new(),
                        });
                        continue 'file_loop;
                    }

                    if opts.output_mode == GrepOutputMode::Count {
                        continue;
                    }

                    let before_lines: Vec<String> = lines
                        [line_idx.saturating_sub(before as usize)..line_idx]
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect();
                    let after_end = (line_idx + 1 + after as usize).min(lines.len());
                    let after_lines: Vec<String> = lines[line_idx + 1..after_end]
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect();

                    let col = if opts.invert {
                        0
                    } else {
                        re.find(line).map_or(0, |m| m.start())
                    };

                    matches.push(GrepMatch {
                        file: file_path.clone(),
                        line_number: if opts.line_numbers { line_idx + 1 } else { 0 },
                        column: col,
                        line_content: line.to_string(),
                        before: before_lines,
                        after: after_lines,
                    });

                    if let Some(max) = opts.max_matches {
                        if total >= max {
                            break 'file_loop;
                        }
                    }
                }

                if opts.output_mode == GrepOutputMode::Count && file_match_count > 0 {
                    matches.push(GrepMatch {
                        file: file_path.clone(),
                        line_number: file_match_count,
                        column: 0,
                        line_content: file_match_count.to_string(),
                        before: Vec::new(),
                        after: Vec::new(),
                    });
                }
            }

            Ok(matches)
        })
    }

    fn glob(&self, pattern: &str) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>> {
        let pattern = pattern.to_string();
        Box::pin(async move {
            let entries = {
                let files = self
                    .files
                    .read()
                    .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
                files
                    .keys()
                    .filter(|p| {
                        let name = p.rsplit('/').next().unwrap_or("");
                        glob_matches(&pattern, name)
                    })
                    .map(|p| GlobEntry {
                        path: p.clone(),
                        is_dir: false,
                        size: files.get(p).map(|v| v.len() as u64),
                    })
                    .collect::<Vec<_>>()
            };
            Ok(entries)
        })
    }

    fn upload(&self, _from: &str, _to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "upload not supported on MemoryProvider".into(),
            ))
        })
    }

    fn download(&self, _from: &str, _to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        Box::pin(async {
            Err(VfsError::Unsupported(
                "download not supported on MemoryProvider".into(),
            ))
        })
    }

    fn pwd(&self) -> BoxFuture<'_, Result<String, VfsError>> {
        Box::pin(async move { self.current_cwd() })
    }

    fn cd(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;

            // Reject `..` that escapes root.
            if resolved != "/" {
                let prefix = format!("{}/", resolved.trim_end_matches('/'));
                // Allow cd to "/" always; for other paths check that at least
                // one entry exists under that prefix OR the path itself exists.
                let exists = self
                    .files
                    .read()
                    .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?
                    .keys()
                    .any(|k| k.starts_with(&prefix) || k == &resolved);
                if !exists {
                    return Err(VfsError::NotFound(resolved));
                }
            }

            *self
                .cwd
                .lock()
                .map_err(|_| VfsError::Unsupported("mutex poisoned".into()))? = resolved;
            Ok(())
        })
    }

    fn head(&self, path: &str, opts: HeadTailOptions) -> BoxFuture<'_, Result<String, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let content = self.read(&path).await?;
            let text = String::from_utf8(content.content)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;
            if let Some(n) = opts.bytes {
                return Ok(text.chars().take(n).collect());
            }
            let n = opts.lines.unwrap_or(10);
            let result: String = text.lines().take(n).collect::<Vec<_>>().join("\n");
            Ok(result)
        })
    }

    fn tail(&self, path: &str, opts: HeadTailOptions) -> BoxFuture<'_, Result<String, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let content = self.read(&path).await?;
            let text = String::from_utf8(content.content)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;
            if let Some(n) = opts.bytes {
                let start = text.len().saturating_sub(n);
                return Ok(text[start..].to_string());
            }
            let n = opts.lines.unwrap_or(10);
            let lines: Vec<&str> = text.lines().collect();
            let start = lines.len().saturating_sub(n);
            Ok(lines[start..].join("\n"))
        })
    }

    fn read_range(&self, path: &str, range: ReadRange) -> BoxFuture<'_, Result<String, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let content = self.read(&path).await?;
            let text = String::from_utf8(content.content)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;

            // Byte range takes precedence over line range.
            if range.byte_start.is_some() || range.byte_end.is_some() {
                let start = range.byte_start.unwrap_or(0).min(text.len());
                let end = range.byte_end.unwrap_or(text.len()).min(text.len());
                let start = start.min(end);
                return Ok(text[start..end].to_string());
            }

            // Line range (1-indexed inclusive).
            if range.line_start.is_some() || range.line_end.is_some() {
                let lines: Vec<&str> = text.lines().collect();
                let start = range
                    .line_start
                    .unwrap_or(1)
                    .saturating_sub(1)
                    .min(lines.len());
                let end = range.line_end.unwrap_or(lines.len()).min(lines.len());
                let end = end.max(start);
                return Ok(lines[start..end].join("\n"));
            }

            // No range constraints — return full content.
            Ok(text)
        })
    }

    fn stat(&self, path: &str) -> BoxFuture<'_, Result<FileInfo, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let files = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
            if let Some(content) = files.get(&resolved) {
                return Ok(FileInfo {
                    path: resolved,
                    size: content.len() as u64,
                    is_dir: false,
                    is_symlink: false,
                    modified: None,
                    permissions: None,
                });
            }
            // Check if it's a directory (prefix of some key).
            let prefix = format!("{}/", resolved.trim_end_matches('/'));
            let is_dir = resolved == "/" || files.keys().any(|k| k.starts_with(&prefix));
            drop(files);
            if is_dir {
                return Ok(FileInfo {
                    path: resolved,
                    size: 0,
                    is_dir: true,
                    is_symlink: false,
                    modified: None,
                    permissions: None,
                });
            }
            Err(VfsError::NotFound(resolved))
        })
    }

    fn wc(&self, path: &str) -> BoxFuture<'_, Result<WordCount, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let content = self.read(&path).await?;
            let bytes = content.content.len();
            let text = String::from_utf8(content.content)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;
            let lines = text.lines().count();
            let words = text.split_whitespace().count();
            let chars = text.chars().count();
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            Ok(WordCount {
                path: resolved,
                lines,
                words,
                bytes,
                chars,
            })
        })
    }

    fn du(&self, path: &str, opts: DuOptions) -> BoxFuture<'_, Result<DiskUsage, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let prefix = if resolved == "/" {
                "/".to_string()
            } else {
                format!("{}/", resolved.trim_end_matches('/'))
            };
            let files = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;

            let mut total_bytes = 0u64;
            let mut entries = Vec::new();
            for (k, v) in files.iter() {
                if !k.starts_with(&prefix) && k != &resolved {
                    continue;
                }
                let size = v.len() as u64;
                total_bytes += size;
                if !opts.summary {
                    let depth = k[prefix.len()..].matches('/').count();
                    if opts.max_depth.is_none() || depth <= opts.max_depth.unwrap_or(0) {
                        entries.push(DiskUsageEntry {
                            path: k.clone(),
                            bytes: size,
                            is_dir: false,
                        });
                    }
                }
            }
            drop(files);
            Ok(DiskUsage {
                path: resolved,
                total_bytes,
                entries,
            })
        })
    }

    fn append(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<WriteResult, VfsError>> {
        let path = path.to_string();
        let content = content.to_vec();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let mut files = self
                .files
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
            let entry = files.entry(resolved.clone()).or_default();
            entry.extend_from_slice(&content);
            drop(files);
            let bytes_written = content.len() as u64;
            Ok(WriteResult {
                path: resolved,
                bytes_written,
            })
        })
    }

    fn mkdir(&self, _path: &str, _opts: MkdirOptions) -> BoxFuture<'_, Result<(), VfsError>> {
        // In-memory provider: directories are implicit (exist when files exist under them).
        // mkdir is a no-op that always succeeds.
        Box::pin(async { Ok(()) })
    }

    fn touch(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let mut files = self
                .files
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
            let _ = files.entry(resolved).or_insert_with(Vec::new);
            drop(files);
            Ok(())
        })
    }

    fn diff(
        &self,
        a: &str,
        b: &str,
        opts: DiffOptions,
    ) -> BoxFuture<'_, Result<DiffResult, VfsError>> {
        let a = a.to_string();
        let b = b.to_string();
        Box::pin(async move {
            let ca = self.read(&a).await?;
            let cb = self.read(&b).await?;
            let ta = String::from_utf8(ca.content)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;
            let tb = String::from_utf8(cb.content)
                .map_err(|_| VfsError::Unsupported("binary file".into()))?;
            if ta == tb {
                return Ok(DiffResult {
                    equal: true,
                    hunks: Vec::new(),
                });
            }
            // Simple line-by-line diff.
            let la: Vec<&str> = ta.lines().collect();
            let lb: Vec<&str> = tb.lines().collect();
            let ctx = opts.context_lines as usize;
            let mut hunks = Vec::new();
            let mut i = 0;
            let mut j = 0;
            while i < la.len() || j < lb.len() {
                if i < la.len() && j < lb.len() && la[i] == lb[j] {
                    i += 1;
                    j += 1;
                    continue;
                }
                // Found a difference — build a hunk.
                let hunk_start_i = i.saturating_sub(ctx);
                let hunk_start_j = j.saturating_sub(ctx);
                let mut lines = Vec::new();
                // Before context.
                for line in la.iter().take(i).skip(hunk_start_i) {
                    lines.push(DiffLine::Context((*line).to_string()));
                }
                // Changed lines.
                while i < la.len()
                    && (j >= lb.len() || (i < la.len() && j < lb.len() && la[i] != lb[j]))
                {
                    lines.push(DiffLine::Removed(la[i].to_string()));
                    i += 1;
                }
                while j < lb.len()
                    && (i >= la.len() || (i < la.len() && j < lb.len() && la.get(i) != lb.get(j)))
                {
                    lines.push(DiffLine::Added(lb[j].to_string()));
                    j += 1;
                }
                // After context.
                let after_end_i = (i + ctx).min(la.len());
                let after_end_j = (j + ctx).min(lb.len());
                let after_count = after_end_i
                    .saturating_sub(i)
                    .min(after_end_j.saturating_sub(j));
                for k in 0..after_count {
                    if i + k < la.len() {
                        lines.push(DiffLine::Context(la[i + k].to_string()));
                    }
                }
                i += after_count;
                j += after_count;
                hunks.push(DiffHunk {
                    old_start: hunk_start_i + 1,
                    old_count: i - hunk_start_i,
                    new_start: hunk_start_j + 1,
                    new_count: j - hunk_start_j,
                    lines,
                });
            }
            Ok(DiffResult {
                equal: false,
                hunks,
            })
        })
    }

    fn rm(&self, path: &str, opts: RmOptions) -> BoxFuture<'_, Result<(), VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let mut files = self
                .files
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;

            if opts.recursive {
                let prefix = format!("{}/", resolved.trim_end_matches('/'));
                let keys: Vec<String> = files
                    .keys()
                    .filter(|k| k.starts_with(&prefix) || *k == &resolved)
                    .cloned()
                    .collect();
                if keys.is_empty() && !opts.force {
                    return Err(VfsError::NotFound(resolved));
                }
                for k in keys {
                    let _ = files.remove(&k);
                }
                drop(files);
            } else if files.remove(&resolved).is_none() && !opts.force {
                return Err(VfsError::NotFound(resolved));
            }
            Ok(())
        })
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
            let cwd = self.current_cwd()?;
            let src = Self::resolve(&cwd, &from)?;
            let dst = Self::resolve(&cwd, &to)?;

            if opts.recursive {
                let prefix = format!("{}/", src.trim_end_matches('/'));
                let files = self
                    .files
                    .read()
                    .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
                let mut copies: Vec<(String, Vec<u8>)> = Vec::new();
                let mut total = 0u64;
                for (k, v) in files.iter() {
                    if k == &src || k.starts_with(&prefix) {
                        let rel = k.strip_prefix(src.trim_end_matches('/')).unwrap_or(k);
                        let new_path = format!("{}{}", dst.trim_end_matches('/'), rel);
                        total += v.len() as u64;
                        copies.push((new_path, v.clone()));
                    }
                }
                drop(files);
                let mut files = self
                    .files
                    .write()
                    .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
                for (path, content) in copies {
                    if opts.no_overwrite && files.contains_key(&path) {
                        continue;
                    }
                    let _ = files.insert(path, content);
                }
                drop(files);
                return Ok(TransferResult {
                    path: dst,
                    bytes_transferred: total,
                });
            }

            let files = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
            let content = files
                .get(&src)
                .cloned()
                .ok_or_else(|| VfsError::NotFound(src.clone()))?;
            drop(files);

            let bytes_transferred = content.len() as u64;
            let mut files = self
                .files
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
            if opts.no_overwrite && files.contains_key(&dst) {
                return Ok(TransferResult {
                    path: dst,
                    bytes_transferred: 0,
                });
            }
            let _ = files.insert(dst.clone(), content);
            drop(files);
            Ok(TransferResult {
                path: dst,
                bytes_transferred,
            })
        })
    }

    fn find(
        &self,
        path: &str,
        opts: FindOptions,
    ) -> BoxFuture<'_, Result<Vec<FindEntry>, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let prefix = if resolved == "/" {
                "/".to_string()
            } else {
                format!("{}/", resolved.trim_end_matches('/'))
            };
            let files = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;

            let mut results = Vec::new();
            let mut seen_dirs = std::collections::HashSet::new();

            for (k, v) in files.iter() {
                if !k.starts_with(&prefix) && k != &resolved {
                    continue;
                }
                let rel = &k[prefix.len()..];
                let depth = rel.matches('/').count();
                if let Some(max) = opts.max_depth {
                    if depth > max {
                        continue;
                    }
                }
                // Collect intermediate directories.
                let parts: Vec<&str> = rel.split('/').collect();
                for i in 0..parts.len().saturating_sub(1) {
                    let dir_path = format!("{}{}", prefix, parts[..=i].join("/"));
                    if seen_dirs.insert(dir_path.clone()) {
                        let dir_name = parts[i];
                        let dir_depth = i;
                        if let Some(max) = opts.max_depth {
                            if dir_depth > max {
                                continue;
                            }
                        }
                        if let Some(ref ft) = opts.entry_type {
                            if *ft != FindType::Directory {
                                continue;
                            }
                        }
                        if let Some(ref name) = opts.name {
                            if !glob_matches(name, dir_name) {
                                continue;
                            }
                        }
                        results.push(FindEntry {
                            path: dir_path,
                            is_dir: true,
                            is_symlink: false,
                            size: None,
                            modified: None,
                        });
                    }
                }
                // The file itself.
                let name = k.rsplit('/').next().unwrap_or("");
                if let Some(ref ft) = opts.entry_type {
                    if *ft != FindType::File {
                        continue;
                    }
                }
                if let Some(ref pat) = opts.name {
                    if !glob_matches(pat, name) {
                        continue;
                    }
                }
                let size = v.len() as u64;
                if let Some(min) = opts.min_size {
                    if size < min {
                        continue;
                    }
                }
                if let Some(max) = opts.max_size {
                    if size > max {
                        continue;
                    }
                }
                results.push(FindEntry {
                    path: k.clone(),
                    is_dir: false,
                    is_symlink: false,
                    size: Some(size),
                    modified: None,
                });
            }
            drop(files);
            Ok(results)
        })
    }

    fn tree(&self, path: &str, opts: TreeOptions) -> BoxFuture<'_, Result<TreeEntry, VfsError>> {
        let path = path.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let resolved = Self::resolve(&cwd, &path)?;
            let files = self
                .files
                .read()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
            let root_name = resolved.rsplit('/').next().unwrap_or("/").to_string();
            let tree = build_tree(&resolved, &root_name, &files, &opts, 0);
            drop(files);
            Ok(tree)
        })
    }

    fn mv_file(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        let from = from.to_string();
        let to = to.to_string();
        Box::pin(async move {
            let cwd = self.current_cwd()?;
            let src = Self::resolve(&cwd, &from)?;
            let dst = Self::resolve(&cwd, &to)?;
            let mut files = self
                .files
                .write()
                .map_err(|_| VfsError::Unsupported("rwlock poisoned".into()))?;
            let content = files
                .remove(&src)
                .ok_or_else(|| VfsError::NotFound(src.clone()))?;
            let bytes_transferred = content.len() as u64;
            let _ = files.insert(dst.clone(), content);
            drop(files);
            Ok(TransferResult {
                path: dst,
                bytes_transferred,
            })
        })
    }

    fn capabilities(&self) -> VfsCapabilities {
        VfsCapabilities::LS
            | VfsCapabilities::READ
            | VfsCapabilities::HEAD
            | VfsCapabilities::TAIL
            | VfsCapabilities::STAT
            | VfsCapabilities::WC
            | VfsCapabilities::DU
            | VfsCapabilities::WRITE
            | VfsCapabilities::APPEND
            | VfsCapabilities::MKDIR
            | VfsCapabilities::TOUCH
            | VfsCapabilities::EDIT
            | VfsCapabilities::DIFF
            | VfsCapabilities::GREP
            | VfsCapabilities::GLOB
            | VfsCapabilities::FIND
            | VfsCapabilities::TREE
            | VfsCapabilities::PWD
            | VfsCapabilities::CD
            | VfsCapabilities::RM
            | VfsCapabilities::CP
            | VfsCapabilities::MV
    }

    fn provider_name(&self) -> &'static str {
        "MemoryProvider"
    }
}

/// Build a recursive tree from the in-memory file map.
fn build_tree(
    dir_path: &str,
    name: &str,
    files: &BTreeMap<String, Vec<u8>>,
    opts: &TreeOptions,
    depth: usize,
) -> TreeEntry {
    let prefix = if dir_path == "/" {
        "/".to_string()
    } else {
        format!("{}/", dir_path.trim_end_matches('/'))
    };

    let mut children_map: BTreeMap<String, Option<u64>> = BTreeMap::new();
    let mut subdirs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (k, v) in files {
        if !k.starts_with(&prefix) {
            continue;
        }
        let rest = &k[prefix.len()..];
        let component = rest.split('/').next().unwrap_or("");
        if component.is_empty() {
            continue;
        }
        if !opts.all && component.starts_with('.') {
            continue;
        }
        if rest.contains('/') {
            let _ = subdirs.insert(component.to_string());
        } else {
            let _ = children_map.insert(component.to_string(), Some(v.len() as u64));
        }
    }

    let at_depth_limit = opts.max_depth.is_some_and(|max| depth >= max);
    let mut children = Vec::new();

    for dir_name in &subdirs {
        let child_path = format!("{prefix}{dir_name}");
        if at_depth_limit {
            children.push(TreeEntry {
                name: dir_name.clone(),
                path: child_path,
                is_dir: true,
                size: None,
                children: Vec::new(),
            });
        } else {
            children.push(build_tree(&child_path, dir_name, files, opts, depth + 1));
        }
    }

    if !opts.dirs_only {
        for (file_name, size) in &children_map {
            if !subdirs.contains(file_name) {
                children.push(TreeEntry {
                    name: file_name.clone(),
                    path: format!("{prefix}{file_name}"),
                    is_dir: false,
                    size: *size,
                    children: Vec::new(),
                });
            }
        }
    }

    TreeEntry {
        name: name.to_string(),
        path: dir_path.to_string(),
        is_dir: true,
        size: None,
        children,
    }
}

/// Simple glob matching: `*` matches any sequence of non-separator chars,
/// `**` matches anything.
///
/// Exported for use by VFS providers in synwire-agent.
pub fn glob_matches_pub(pattern: &str, name: &str) -> bool {
    glob_matches(pattern, name)
}

fn glob_matches(pattern: &str, name: &str) -> bool {
    if pattern == "**" || pattern == "*" {
        return true;
    }
    // Build regex from glob.
    let mut regex = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str("[^/]*"),
            '?' => regex.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            c => regex.push(c),
        }
    }
    regex.push('$');
    Regex::new(&regex).is_ok_and(|re| re.is_match(name))
}

/// Map ripgrep-style file type names to extensions.
///
/// Exported for use by VFS providers in synwire-agent.
pub fn matches_file_type_pub(file_type: &str, ext: &str) -> bool {
    matches_file_type(file_type, ext)
}

fn matches_file_type(file_type: &str, ext: &str) -> bool {
    match file_type {
        "rust" | "rs" => ext == "rs",
        "python" | "py" => ext == "py",
        "js" | "javascript" => ext == "js" || ext == "mjs" || ext == "cjs",
        "ts" | "typescript" => ext == "ts" || ext == "tsx",
        "json" => ext == "json",
        "yaml" | "yml" => ext == "yaml" || ext == "yml",
        "toml" => ext == "toml",
        "md" | "markdown" => ext == "md" || ext == "markdown",
        "go" => ext == "go",
        "sh" | "bash" => ext == "sh" || ext == "bash",
        _ => file_type == ext,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::case_sensitive_file_extension_comparisons
)]
mod tests {
    use super::*;

    fn backend_with_files(files: &[(&str, &str)]) -> MemoryProvider {
        let backend = MemoryProvider::new();
        for (path, content) in files {
            let mut store = backend.files.write().expect("lock");
            let _ = store.insert(path.to_string(), content.as_bytes().to_vec());
        }
        backend
    }

    // ── grep tests ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_grep_with_context() {
        let backend = backend_with_files(&[("/file.txt", "line1\nline2\nMATCH\nline4\nline5")]);
        let opts = GrepOptions {
            context: Some(3),
            line_numbers: true,
            ..Default::default()
        };
        let results = backend.grep("MATCH", opts).await.expect("grep");
        assert!(!results.is_empty());
        let m = &results[0];
        // Before context: up to 3 lines before MATCH.
        assert!(m.before.len() <= 3);
        assert!(m.after.len() <= 3);
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let backend = backend_with_files(&[("/f.txt", "Hello World")]);
        let opts = GrepOptions {
            case_insensitive: true,
            ..Default::default()
        };
        let results = backend.grep("hello", opts).await.expect("grep");
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_grep_file_type_filter() {
        let backend = backend_with_files(&[
            ("/src/main.rs", "fn main() {}"),
            ("/src/main.py", "def main(): pass"),
        ]);
        let opts = GrepOptions {
            file_type: Some("rust".into()),
            ..Default::default()
        };
        let results = backend.grep("main", opts).await.expect("grep");
        assert!(results.iter().all(|m| m.file.ends_with(".rs")));
    }

    #[tokio::test]
    async fn test_grep_invert_match() {
        let backend = backend_with_files(&[("/f.txt", "apple\nbanana\ncherry")]);
        let opts = GrepOptions {
            invert: true,
            ..Default::default()
        };
        let results = backend.grep("banana", opts).await.expect("grep");
        for m in &results {
            assert!(!m.line_content.contains("banana"));
        }
    }

    #[tokio::test]
    async fn test_grep_count_mode() {
        let backend = backend_with_files(&[("/f.txt", "foo\nfoo\nbar")]);
        let opts = GrepOptions {
            output_mode: GrepOutputMode::Count,
            ..Default::default()
        };
        let results = backend.grep("foo", opts).await.expect("grep");
        // line_number field holds the count.
        assert_eq!(results[0].line_number, 2);
    }

    #[tokio::test]
    async fn test_grep_max_matches() {
        let backend = backend_with_files(&[("/f.txt", "a\na\na\na\na")]);
        let opts = GrepOptions {
            max_matches: Some(2),
            ..Default::default()
        };
        let results = backend.grep("a", opts).await.expect("grep");
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn test_grep_skips_binary_files() {
        let backend = MemoryProvider::new();
        {
            let mut store = backend.files.write().expect("lock");
            let _ = store.insert("/bin.dat".to_string(), vec![0u8, 1, 2, 3]);
        }
        let results = backend
            .grep(".", GrepOptions::default())
            .await
            .expect("grep");
        assert!(results.iter().all(|m| m.file != "/bin.dat"));
    }

    #[tokio::test]
    async fn test_grep_line_numbers() {
        let backend = backend_with_files(&[("/f.txt", "a\nb\nMATCH\nd")]);
        let opts = GrepOptions {
            line_numbers: true,
            ..Default::default()
        };
        let results = backend.grep("MATCH", opts).await.expect("grep");
        assert_eq!(results[0].line_number, 3);
    }

    // ── cd / pwd tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_cd_pwd_roundtrip() {
        let backend = backend_with_files(&[("/home/user/file.txt", "hi")]);
        backend.cd("/home/user").await.expect("cd");
        let cwd = backend.pwd().await.expect("pwd");
        assert_eq!(cwd, "/home/user");
    }

    #[tokio::test]
    async fn test_relative_path_resolution() {
        let backend = backend_with_files(&[("/a/b/c.txt", "data")]);
        backend.cd("/a").await.expect("cd");
        let content = backend.read("b/c.txt").await.expect("read relative");
        assert_eq!(content.content, b"data");
    }

    #[tokio::test]
    async fn test_cd_to_nonexistent_fails_without_state_change() {
        let backend = MemoryProvider::new();
        let err = backend.cd("/nonexistent").await.expect_err("should fail");
        assert!(matches!(err, VfsError::NotFound(_)));
        let cwd = backend.pwd().await.expect("pwd");
        assert_eq!(cwd, "/"); // unchanged
    }

    #[tokio::test]
    async fn test_cd_parent_traversal_rejected() {
        let backend = MemoryProvider::new();
        let err = backend.cd("/../etc").await.expect_err("traversal");
        assert!(matches!(err, VfsError::PathTraversal { .. }));
    }
}

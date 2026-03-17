//! Virtual filesystem trait.

use crate::BoxFuture;
use crate::vfs::error::VfsError;
use crate::vfs::grep_options::GrepOptions;
use crate::vfs::types::{
    CommunityEntry, CommunityMembersResult, CommunitySearchOptions, CommunitySearchResult,
    CommunitySummaryResult, CpOptions, DiffOptions, DiffResult, DirEntry, DiskUsage, DuOptions,
    EditResult, FileContent, FileInfo, FindEntry, FindOptions, GlobEntry, GrepMatch,
    HeadTailOptions, HybridSearchOptions, HybridSearchResult, IndexHandle, IndexOptions,
    IndexStatus, LsOptions, MkdirOptions, MountInfo, ReadRange, RmOptions, SemanticSearchOptions,
    SemanticSearchResult, TransferResult, TreeEntry, TreeOptions, VfsCapabilities, WordCount,
    WriteResult,
};

/// Virtual filesystem interface for agent data operations.
///
/// Provides a filesystem-like abstraction over heterogeneous data sources,
/// allowing LLMs to interact with any data source using familiar operations
/// (ls, read, write, cd, cp, mv, etc.).
///
/// Operations mirror Linux coreutils with their most useful arguments
/// abstracted into option structs.  Providers declare which operations
/// they support via [`VfsCapabilities`].  Unsupported operations return
/// [`VfsError::Unsupported`] by default — override to opt in.
///
/// All implementations must be `Send + Sync` and return `BoxFuture` results.
pub trait Vfs: Send + Sync {
    // ── Navigation ───────────────────────────────────────────────────────

    /// Return the current working directory.  `pwd`
    fn pwd(&self) -> BoxFuture<'_, Result<String, VfsError>>;

    /// Change the current working directory.  `cd <path>`
    fn cd(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>>;

    // ── Listing ──────────────────────────────────────────────────────────

    /// List directory contents.  `ls [opts] <path>`
    ///
    /// Options: `-a` all, `-l` long, `-R` recursive, `-S`/`-t` sort, `-r` reverse.
    fn ls(&self, path: &str, opts: LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>>;

    /// Recursive directory tree.  `tree [opts] <path>`
    ///
    /// Options: `-L` max depth, `-d` dirs only, `-a` all.
    fn tree(&self, path: &str, opts: TreeOptions) -> BoxFuture<'_, Result<TreeEntry, VfsError>> {
        let _ = (path, opts);
        Box::pin(async { Err(VfsError::Unsupported("tree".into())) })
    }

    // ── Reading ──────────────────────────────────────────────────────────

    /// Read entire file contents.  `cat <path>`
    fn read(&self, path: &str) -> BoxFuture<'_, Result<FileContent, VfsError>>;

    /// Read a sub-range of a file by line numbers or byte offsets.
    ///
    /// Line numbers are 1-indexed.  Byte offsets are 0-indexed.
    /// If both line and byte ranges are specified, byte range takes precedence.
    fn read_range(&self, path: &str, range: ReadRange) -> BoxFuture<'_, Result<String, VfsError>> {
        let _ = (path, range);
        Box::pin(async { Err(VfsError::Unsupported("read_range".into())) })
    }

    /// Read the first N lines or bytes.  `head [opts] <path>`
    ///
    /// Options: `-n` lines, `-c` bytes.  Default: 10 lines.
    fn head(&self, path: &str, opts: HeadTailOptions) -> BoxFuture<'_, Result<String, VfsError>> {
        let _ = (path, opts);
        Box::pin(async { Err(VfsError::Unsupported("head".into())) })
    }

    /// Read the last N lines or bytes.  `tail [opts] <path>`
    ///
    /// Options: `-n` lines, `-c` bytes.  Default: 10 lines.
    fn tail(&self, path: &str, opts: HeadTailOptions) -> BoxFuture<'_, Result<String, VfsError>> {
        let _ = (path, opts);
        Box::pin(async { Err(VfsError::Unsupported("tail".into())) })
    }

    /// File metadata.  `stat <path>`
    fn stat(&self, path: &str) -> BoxFuture<'_, Result<FileInfo, VfsError>> {
        let _ = path;
        Box::pin(async { Err(VfsError::Unsupported("stat".into())) })
    }

    /// Line, word, and byte counts.  `wc <path>`
    fn wc(&self, path: &str) -> BoxFuture<'_, Result<WordCount, VfsError>> {
        let _ = path;
        Box::pin(async { Err(VfsError::Unsupported("wc".into())) })
    }

    /// Disk usage.  `du [opts] <path>`
    ///
    /// Options: `-s` summary, `-d` max depth.
    fn du(&self, path: &str, opts: DuOptions) -> BoxFuture<'_, Result<DiskUsage, VfsError>> {
        let _ = (path, opts);
        Box::pin(async { Err(VfsError::Unsupported("du".into())) })
    }

    // ── Writing ──────────────────────────────────────────────────────────

    /// Write bytes to a file (creates or overwrites).  `>`
    fn write(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<WriteResult, VfsError>>;

    /// Append bytes to a file (creates if absent).  `>>`
    fn append(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<WriteResult, VfsError>> {
        let _ = (path, content);
        Box::pin(async { Err(VfsError::Unsupported("append".into())) })
    }

    /// Create a directory.  `mkdir [opts] <path>`
    ///
    /// Options: `-p` create parents, `-m` mode.
    fn mkdir(&self, path: &str, opts: MkdirOptions) -> BoxFuture<'_, Result<(), VfsError>> {
        let _ = (path, opts);
        Box::pin(async { Err(VfsError::Unsupported("mkdir".into())) })
    }

    /// Create an empty file or update its timestamp.  `touch <path>`
    fn touch(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let _ = path;
        Box::pin(async { Err(VfsError::Unsupported("touch".into())) })
    }

    // ── Editing ──────────────────────────────────────────────────────────

    /// Edit a file by replacing `old` with `new` (first occurrence).  `sed`-like.
    fn edit(&self, path: &str, old: &str, new: &str)
    -> BoxFuture<'_, Result<EditResult, VfsError>>;

    /// Compare two files.  `diff [opts] <a> <b>`
    ///
    /// Options: `-U` context lines.
    fn diff(
        &self,
        a: &str,
        b: &str,
        opts: DiffOptions,
    ) -> BoxFuture<'_, Result<DiffResult, VfsError>> {
        let _ = (a, b, opts);
        Box::pin(async { Err(VfsError::Unsupported("diff".into())) })
    }

    // ── File management ──────────────────────────────────────────────────

    /// Remove a file or directory.  `rm [opts] <path>`
    ///
    /// Options: `-r` recursive, `-f` force.
    fn rm(&self, path: &str, opts: RmOptions) -> BoxFuture<'_, Result<(), VfsError>>;

    /// Copy `from` to `to`.  `cp [opts] <from> <to>`
    ///
    /// Options: `-r` recursive, `-n` no-overwrite.
    fn cp(
        &self,
        from: &str,
        to: &str,
        opts: CpOptions,
    ) -> BoxFuture<'_, Result<TransferResult, VfsError>>;

    /// Move / rename `from` to `to`.  `mv <from> <to>`
    fn mv_file(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>>;

    /// Create a link.  `ln [-s] <target> <link>`
    fn ln(&self, target: &str, link: &str, symbolic: bool) -> BoxFuture<'_, Result<(), VfsError>> {
        let _ = (target, link, symbolic);
        Box::pin(async { Err(VfsError::Unsupported("ln".into())) })
    }

    /// Change file permissions.  `chmod <mode> <path>`
    fn chmod(&self, path: &str, mode: u32) -> BoxFuture<'_, Result<(), VfsError>> {
        let _ = (path, mode);
        Box::pin(async { Err(VfsError::Unsupported("chmod".into())) })
    }

    // ── Search ───────────────────────────────────────────────────────────

    /// Search file contents.  `grep [opts] <pattern>`
    fn grep(
        &self,
        pattern: &str,
        opts: GrepOptions,
    ) -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>>;

    /// Glob for file paths matching a pattern.
    fn glob(&self, pattern: &str) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>>;

    /// Search for files by criteria.  `find [opts] <path>`
    ///
    /// Options: `-name`, `-type`, `-maxdepth`, `-size`, `-newer`.
    fn find(
        &self,
        path: &str,
        opts: FindOptions,
    ) -> BoxFuture<'_, Result<Vec<FindEntry>, VfsError>> {
        let _ = (path, opts);
        Box::pin(async { Err(VfsError::Unsupported("find".into())) })
    }

    // ── Transfer ─────────────────────────────────────────────────────────

    /// Upload a file from `from` (local path) to `to` (VFS path).
    fn upload(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>>;

    /// Download a file from `from` (VFS path) to `to` (local path).
    fn download(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>>;

    // ── Watch ─────────────────────────────────────────────────────────

    /// Begin watching a path for external modifications.
    ///
    /// Called automatically by VFS tools after every read.  Providers
    /// that support watching record the file's current state (mtime,
    /// content hash, etc.) so that [`check_stale`](Self::check_stale)
    /// can detect external changes before an edit or write.
    fn watch(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let _ = path;
        Box::pin(async { Ok(()) })
    }

    /// Check whether a previously-read file has been modified externally.
    ///
    /// Returns `Ok(())` if the file is unchanged since the last
    /// [`watch`](Self::watch) call.  Returns
    /// [`VfsError::StaleRead`] if the file was modified.  Returns
    /// `Ok(())` for paths that were never watched (the tool layer
    /// enforces the "must read before edit" rule separately).
    fn check_stale(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        let _ = path;
        Box::pin(async { Ok(()) })
    }

    // ── Semantic index / search ─────────────────────────────────────────

    /// Start indexing a directory for semantic search.
    ///
    /// Returns immediately with an [`IndexHandle`] — indexing runs in the
    /// background.  Poll with [`index_status`](Self::index_status) or
    /// subscribe to [`IndexEvent`](crate::vfs::types::IndexEvent)s.
    ///
    /// Denies indexing `/` (root filesystem) with [`VfsError::IndexDenied`].
    fn index(
        &self,
        path: &str,
        opts: IndexOptions,
    ) -> BoxFuture<'_, Result<IndexHandle, VfsError>> {
        let _ = (path, opts);
        Box::pin(async { Err(VfsError::Unsupported("index".into())) })
    }

    /// Check the status of an indexing operation.
    fn index_status(&self, index_id: &str) -> BoxFuture<'_, Result<IndexStatus, VfsError>> {
        let _ = index_id;
        Box::pin(async { Err(VfsError::Unsupported("index_status".into())) })
    }

    /// Semantic search across indexed content.
    ///
    /// Returns ranked results with file paths, line ranges, content, and
    /// similarity scores.  Returns [`VfsError::IndexNotReady`] if the
    /// index is still building.
    fn semantic_search(
        &self,
        query: &str,
        opts: SemanticSearchOptions,
    ) -> BoxFuture<'_, Result<Vec<SemanticSearchResult>, VfsError>> {
        let _ = (query, opts);
        Box::pin(async { Err(VfsError::Unsupported("semantic_search".into())) })
    }

    /// Hybrid BM25 + vector search across indexed content.
    ///
    /// Combines keyword recall (BM25) with semantic similarity (vector) for
    /// higher-quality results than either signal alone.  The `alpha` parameter
    /// in [`HybridSearchOptions`] controls the blend.
    ///
    /// Returns [`VfsError::Unsupported`] by default.  Providers that back the
    /// `synwire-index` crate with the `hybrid-search` feature should override
    /// this method.
    fn hybrid_search(
        &self,
        query: &str,
        opts: HybridSearchOptions,
    ) -> BoxFuture<'_, Result<Vec<HybridSearchResult>, VfsError>> {
        let _ = (query, opts);
        Box::pin(async { Err(VfsError::Unsupported("hybrid_search".into())) })
    }

    // ── Code navigation ──────────────────────────────────────────────────

    /// Return only the function and method signatures of a source file,
    /// stripping all body content.
    ///
    /// Each signature line is prefixed with its 1-indexed line number so that
    /// the LLM can navigate to the relevant section with `read_range`.
    /// This dramatically reduces token usage compared with reading an entire
    /// file when only the API surface is needed.
    ///
    /// The default implementation returns the full file content unchanged,
    /// which is always safe but provides no token savings.  Override this
    /// method in providers that have access to a tree-sitter grammar for the
    /// file's language.
    ///
    /// Returns [`VfsError::Unsupported`] for binary files.
    fn skeleton<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Result<String, VfsError>> {
        // Default: read the file and return its full text content.
        // Providers with tree-sitter support should override this.
        Box::pin(async move {
            let content = self.read(path).await?;
            String::from_utf8(content.content)
                .map_err(|_| VfsError::Unsupported("skeleton: binary file".into()))
        })
    }

    // ── Community detection ───────────────────────────────────────────────────

    /// List all detected communities with their member counts.
    ///
    /// Returns [`VfsError::Unsupported`] by default.  Override in providers
    /// backed by a community-detection pipeline.
    fn list_communities(&self) -> BoxFuture<'_, Result<Vec<CommunityEntry>, VfsError>> {
        Box::pin(async { Err(VfsError::Unsupported("list_communities".into())) })
    }

    /// List the symbol members of a specific community.
    ///
    /// Returns [`VfsError::Unsupported`] by default.
    fn community_members(
        &self,
        community_id: u64,
    ) -> BoxFuture<'_, Result<CommunityMembersResult, VfsError>> {
        let _ = community_id;
        Box::pin(async { Err(VfsError::Unsupported("community_members".into())) })
    }

    /// Search for communities whose member names match `query`.
    ///
    /// Performs a simple substring / keyword search against member names.
    /// Returns [`VfsError::Unsupported`] by default.
    fn community_search(
        &self,
        query: &str,
        opts: CommunitySearchOptions,
    ) -> BoxFuture<'_, Result<Vec<CommunitySearchResult>, VfsError>> {
        let _ = (query, opts);
        Box::pin(async { Err(VfsError::Unsupported("community_search".into())) })
    }

    /// Get (or generate) a natural-language summary for a community.
    ///
    /// Returns [`VfsError::Unsupported`] by default.  Providers with an
    /// embedded [`SamplingProvider`](crate::agents::sampling::SamplingProvider)
    /// should override this to return an LLM-generated summary.
    fn community_summary(
        &self,
        community_id: u64,
    ) -> BoxFuture<'_, Result<CommunitySummaryResult, VfsError>> {
        let _ = community_id;
        Box::pin(async { Err(VfsError::Unsupported("community_summary".into())) })
    }

    // ── Capabilities & identity ────────────────────────────────────────

    /// Return the capabilities supported by this provider.
    fn capabilities(&self) -> VfsCapabilities;

    /// Human-readable provider type name (e.g. `"LocalProvider"`, `"MemoryProvider"`).
    ///
    /// Used by the `mount` tool to inform the LLM what kind of data
    /// source it is interacting with.
    fn provider_name(&self) -> &'static str {
        "UnknownProvider"
    }

    /// Return mount information for this provider.
    ///
    /// Simple providers return a single entry at `/`.
    /// `CompositeProvider` returns one entry per mount.
    fn mount_info(&self) -> Vec<MountInfo> {
        let caps = self.capabilities();
        let cap_names: Vec<String> = capability_names(caps);
        vec![MountInfo {
            prefix: "/".to_string(),
            provider: self.provider_name().to_string(),
            capabilities: cap_names,
        }]
    }
}

/// Convert capability bitflags to a list of human-readable names.
pub fn capability_names(caps: VfsCapabilities) -> Vec<String> {
    let mut names = Vec::new();
    let flags = [
        (VfsCapabilities::PWD, "fs.pwd"),
        (VfsCapabilities::CD, "fs.cd"),
        (VfsCapabilities::LS, "fs.ls"),
        (VfsCapabilities::TREE, "fs.tree"),
        (VfsCapabilities::READ, "fs.read"),
        (VfsCapabilities::HEAD, "fs.head"),
        (VfsCapabilities::TAIL, "fs.tail"),
        (VfsCapabilities::STAT, "fs.stat"),
        (VfsCapabilities::WC, "fs.wc"),
        (VfsCapabilities::DU, "fs.du"),
        (VfsCapabilities::WRITE, "fs.write"),
        (VfsCapabilities::APPEND, "fs.append"),
        (VfsCapabilities::MKDIR, "fs.mkdir"),
        (VfsCapabilities::TOUCH, "fs.touch"),
        (VfsCapabilities::EDIT, "fs.edit"),
        (VfsCapabilities::DIFF, "fs.diff"),
        (VfsCapabilities::RM, "fs.rm"),
        (VfsCapabilities::CP, "fs.cp"),
        (VfsCapabilities::MV, "fs.mv"),
        (VfsCapabilities::LN, "fs.ln"),
        (VfsCapabilities::CHMOD, "fs.chmod"),
        (VfsCapabilities::GREP, "fs.grep"),
        (VfsCapabilities::GLOB, "fs.glob"),
        (VfsCapabilities::FIND, "fs.find"),
        (VfsCapabilities::UPLOAD, "fs.upload"),
        (VfsCapabilities::DOWNLOAD, "fs.download"),
        (VfsCapabilities::EXEC, "fs.exec"),
        (VfsCapabilities::WATCH, "fs.watch"),
        (VfsCapabilities::INDEX, "index.build"),
        (VfsCapabilities::SEMANTIC_SEARCH, "code.search_semantic"),
        (VfsCapabilities::SKELETON, "fs.skeleton"),
        (VfsCapabilities::HYBRID_SEARCH, "code.search_hybrid"),
        (VfsCapabilities::LIST_COMMUNITIES, "code.list_communities"),
        (VfsCapabilities::COMMUNITY_MEMBERS, "code.community_members"),
        (
            VfsCapabilities::COMMUNITY_SEARCH,
            "code.search_by_community",
        ),
        (VfsCapabilities::COMMUNITY_SUMMARY, "code.community_summary"),
    ];
    for (flag, name) in flags {
        if caps.contains(flag) {
            names.push((*name).to_string());
        }
    }
    names
}

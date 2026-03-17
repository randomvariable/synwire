//! VFS response, capability, and option types.

use bitflags::bitflags;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

bitflags! {
    /// VFS capabilities bitflags.
    ///
    /// Providers declare which operations they support.  Callers can check
    /// before invoking to avoid `VfsError::Unsupported`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VfsCapabilities: u64 {
        // ── Navigation ───────────────────────────────────────────────────
        /// Get working directory (`pwd`).
        const PWD = 1 << 0;
        /// Change working directory (`cd`).
        const CD = 1 << 1;

        // ── Listing ──────────────────────────────────────────────────────
        /// List directory contents (`ls`).
        const LS = 1 << 2;
        /// Recursive directory tree (`tree`).
        const TREE = 1 << 3;

        // ── Reading ──────────────────────────────────────────────────────
        /// Read file contents (`cat`).
        const READ = 1 << 4;
        /// Read first N lines/bytes (`head`).
        const HEAD = 1 << 5;
        /// Read last N lines/bytes (`tail`).
        const TAIL = 1 << 6;
        /// File metadata (`stat`).
        const STAT = 1 << 7;
        /// Line/word/byte counts (`wc`).
        const WC = 1 << 8;
        /// Disk usage (`du`).
        const DU = 1 << 9;

        // ── Writing ──────────────────────────────────────────────────────
        /// Write files (`write` / `>`).
        const WRITE = 1 << 10;
        /// Append to files (`append` / `>>`).
        const APPEND = 1 << 11;
        /// Create directories (`mkdir`).
        const MKDIR = 1 << 12;
        /// Create or update timestamps (`touch`).
        const TOUCH = 1 << 13;

        // ── Editing ──────────────────────────────────────────────────────
        /// In-place text replacement (`edit` / `sed`).
        const EDIT = 1 << 14;
        /// Compare files (`diff`).
        const DIFF = 1 << 15;

        // ── File management ──────────────────────────────────────────────
        /// Remove files/directories (`rm`).
        const RM = 1 << 16;
        /// Copy files/directories (`cp`).
        const CP = 1 << 17;
        /// Move / rename (`mv`).
        const MV = 1 << 18;
        /// Create hard/symbolic links (`ln`).
        const LN = 1 << 19;
        /// Change permissions (`chmod`).
        const CHMOD = 1 << 20;

        // ── Search ───────────────────────────────────────────────────────
        /// Content search (`grep`).
        const GREP = 1 << 21;
        /// Filename pattern matching (`glob`).
        const GLOB = 1 << 22;
        /// Rich file search (`find`).
        const FIND = 1 << 23;

        // ── Transfer ─────────────────────────────────────────────────────
        /// Upload from local to VFS.
        const UPLOAD = 1 << 24;
        /// Download from VFS to local.
        const DOWNLOAD = 1 << 25;

        // ── Execution ────────────────────────────────────────────────────
        /// Execute commands (sandbox protocol).
        const EXEC = 1 << 26;

        // ── Watch ────────────────────────────────────────────────────────
        /// File change detection (`watch` / `check_stale`).
        const WATCH = 1 << 27;

        // ── Semantic ─────────────────────────────────────────────────────
        /// Build semantic indices (`index`).
        const INDEX = 1 << 28;
        /// Semantic search across indexed content (`semantic_search`).
        const SEMANTIC_SEARCH = 1 << 29;

        // ── Code navigation ───────────────────────────────────────────────
        /// Return function/method signatures without bodies (`skeleton`).
        const SKELETON = 1 << 30;

        // ── Hybrid search ─────────────────────────────────────────────────
        /// Hybrid BM25 + vector search (`hybrid_search`).
        const HYBRID_SEARCH = 1 << 31;

        // ── Community detection ───────────────────────────────────────────
        /// List all detected communities (`list_communities`).
        const LIST_COMMUNITIES = 1 << 32;
        /// List members of a specific community (`community_members`).
        const COMMUNITY_MEMBERS = 1 << 33;
        /// Search communities by keyword (`community_search`).
        const COMMUNITY_SEARCH = 1 << 34;
        /// Get or generate a community summary (`community_summary`).
        const COMMUNITY_SUMMARY = 1 << 35;
    }
}

// ── Option structs ───────────────────────────────────────────────────────────

/// Options for `ls` — inspired by `ls(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(clippy::struct_excessive_bools)]
pub struct LsOptions {
    /// Show hidden files (names starting with `.`).  `-a`
    pub all: bool,
    /// Include detailed metadata (size, modified, permissions).  `-l`
    pub long: bool,
    /// Recurse into subdirectories.  `-R`
    pub recursive: bool,
    /// Sort field.  Default is by name.
    pub sort: SortField,
    /// Reverse sort order.  `-r`
    pub reverse: bool,
}

/// Sort field for directory listings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SortField {
    /// Sort by name (default).
    #[default]
    Name,
    /// Sort by size.  `-S`
    Size,
    /// Sort by modification time.  `-t`
    Time,
    /// No sorting — return in provider-native order.  `-U`
    None,
}

/// Options for `rm` — inspired by `rm(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RmOptions {
    /// Remove directories and their contents recursively.  `-r`
    pub recursive: bool,
    /// Ignore nonexistent files (no error).  `-f`
    pub force: bool,
}

/// Options for `cp` — inspired by `cp(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CpOptions {
    /// Copy directories recursively.  `-r`
    pub recursive: bool,
    /// Do not overwrite existing files.  `-n`
    pub no_overwrite: bool,
}

/// Options for `mkdir` — inspired by `mkdir(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MkdirOptions {
    /// Create parent directories as needed.  `-p`
    pub parents: bool,
    /// Set directory permissions (Unix mode).  `-m`
    pub mode: Option<u32>,
}

/// Range selector for partial file reads.
///
/// Specify either a line range or a byte range (not both).
/// Line numbers are 1-indexed.  Byte offsets are 0-indexed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReadRange {
    /// Start line (1-indexed, inclusive).  Omit to start from beginning.
    pub line_start: Option<usize>,
    /// End line (1-indexed, inclusive).  Omit to read to end.
    pub line_end: Option<usize>,
    /// Start byte offset (0-indexed, inclusive).  Takes precedence over line range.
    pub byte_start: Option<usize>,
    /// End byte offset (0-indexed, exclusive).  Takes precedence over line range.
    pub byte_end: Option<usize>,
}

/// Options for `head` and `tail` — inspired by `head(1)` / `tail(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HeadTailOptions {
    /// Number of lines to return.  `-n`
    pub lines: Option<usize>,
    /// Number of bytes to return.  `-c`  Takes precedence over `lines`.
    pub bytes: Option<usize>,
}

/// Options for `find` — inspired by `find(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FindOptions {
    /// Match file name against glob pattern.  `-name`
    pub name: Option<String>,
    /// Filter by entry type.  `-type`
    pub entry_type: Option<FindType>,
    /// Maximum directory depth.  `-maxdepth`
    pub max_depth: Option<usize>,
    /// Minimum file size in bytes.  `-size +N`
    pub min_size: Option<u64>,
    /// Maximum file size in bytes.  `-size -N`
    pub max_size: Option<u64>,
    /// Modified after this time.  `-newer`
    pub newer_than: Option<DateTime<Utc>>,
    /// Modified before this time.
    pub older_than: Option<DateTime<Utc>>,
}

/// File type filter for `find`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FindType {
    /// Regular file (`-type f`).
    File,
    /// Directory (`-type d`).
    Directory,
    /// Symbolic link (`-type l`).
    Symlink,
}

/// Options for `tree` — inspired by `tree(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TreeOptions {
    /// Maximum depth to display.  `-L`
    pub max_depth: Option<usize>,
    /// Only show directories.  `-d`
    pub dirs_only: bool,
    /// Show hidden entries.  `-a`
    pub all: bool,
}

/// Options for `du` — inspired by `du(1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DuOptions {
    /// Show only the total.  `-s`
    pub summary: bool,
    /// Maximum depth for per-directory output.  `-d`
    pub max_depth: Option<usize>,
}

/// Options for `diff` — inspired by `diff(1)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DiffOptions {
    /// Number of context lines around each change.  `-U`
    pub context_lines: u32,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self { context_lines: 3 }
    }
}

// ── Response types ───────────────────────────────────────────────────────────

/// Directory listing entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    /// Entry name.
    pub name: String,
    /// Full path.
    pub path: String,
    /// Is directory.
    pub is_dir: bool,
    /// Size in bytes (None for directories or when `-l` not requested).
    pub size: Option<u64>,
    /// Last modified timestamp (None when `-l` not requested).
    pub modified: Option<DateTime<Utc>>,
    /// Unix permissions (None when not available or `-l` not requested).
    pub permissions: Option<u32>,
    /// Is symlink.
    pub is_symlink: bool,
}

/// File content response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    /// File content as bytes.
    pub content: Vec<u8>,
    /// MIME type if detected.
    pub mime_type: Option<String>,
}

/// Write operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    /// Written file path.
    pub path: String,
    /// Bytes written.
    pub bytes_written: u64,
}

/// Edit operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    /// Edited file path.
    pub path: String,
    /// Number of edits applied.
    pub edits_applied: usize,
    /// Content after edits (optional).
    pub content_after: Option<String>,
}

/// Grep match with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    /// File path.
    pub file: String,
    /// Line number (1-indexed).
    pub line_number: usize,
    /// Column number (0-indexed).
    pub column: usize,
    /// Matched line content.
    pub line_content: String,
    /// Lines before match.
    pub before: Vec<String>,
    /// Lines after match.
    pub after: Vec<String>,
}

/// Glob match entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobEntry {
    /// Matched path.
    pub path: String,
    /// Is directory.
    pub is_dir: bool,
    /// Size in bytes (None for directories).
    pub size: Option<u64>,
}

/// File transfer result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    /// Transferred file path.
    pub path: String,
    /// Bytes transferred.
    pub bytes_transferred: u64,
}

/// File metadata — returned by `stat`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// Is directory.
    pub is_dir: bool,
    /// Is symlink.
    pub is_symlink: bool,
    /// Last modified timestamp.
    pub modified: Option<DateTime<Utc>>,
    /// File permissions (Unix mode).
    pub permissions: Option<u32>,
}

/// Word / line / byte counts — returned by `wc`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCount {
    /// File path.
    pub path: String,
    /// Number of lines.
    pub lines: usize,
    /// Number of words.
    pub words: usize,
    /// Number of bytes.
    pub bytes: usize,
    /// Number of characters (may differ from bytes for multi-byte encodings).
    pub chars: usize,
}

/// Disk usage — returned by `du`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    /// Root path measured.
    pub path: String,
    /// Total bytes used.
    pub total_bytes: u64,
    /// Per-entry breakdown (empty when `DuOptions::summary` is true).
    pub entries: Vec<DiskUsageEntry>,
}

/// A single entry in disk usage output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsageEntry {
    /// Entry path.
    pub path: String,
    /// Bytes used.
    pub bytes: u64,
    /// Is directory.
    pub is_dir: bool,
}

/// Recursive directory tree — returned by `tree`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Entry name.
    pub name: String,
    /// Full path.
    pub path: String,
    /// Is directory.
    pub is_dir: bool,
    /// Size in bytes (None for directories).
    pub size: Option<u64>,
    /// Children (only for directories).
    pub children: Vec<Self>,
}

/// Find result entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindEntry {
    /// Full path.
    pub path: String,
    /// Is directory.
    pub is_dir: bool,
    /// Is symlink.
    pub is_symlink: bool,
    /// Size in bytes (None for directories).
    pub size: Option<u64>,
    /// Last modified timestamp.
    pub modified: Option<DateTime<Utc>>,
}

/// Unified diff result — returned by `diff`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    /// Whether the two files are identical.
    pub equal: bool,
    /// Diff hunks (empty when `equal` is true).
    pub hunks: Vec<DiffHunk>,
}

/// A single hunk in a unified diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// Start line in the old file (1-indexed).
    pub old_start: usize,
    /// Number of lines in the old range.
    pub old_count: usize,
    /// Start line in the new file (1-indexed).
    pub new_start: usize,
    /// Number of lines in the new range.
    pub new_count: usize,
    /// Diff lines in this hunk.
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff hunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DiffLine {
    /// Unchanged context line.
    Context(String),
    /// Line added in the new file.
    Added(String),
    /// Line removed from the old file.
    Removed(String),
}

// ── Mount / provider info ────────────────────────────────────────────────────

/// Information about a single mount point — returned by `mount`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    /// Mount path prefix (e.g. `/workspace`, `/scratch`).
    pub prefix: String,
    /// Human-readable provider type (e.g. `"LocalProvider"`, `"MemoryProvider"`).
    pub provider: String,
    /// Capabilities this mount supports.
    pub capabilities: Vec<String>,
}

// ── Semantic index / search ───────────────────────────────────────────────────

/// Options for `index`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IndexOptions {
    /// Force re-index even if the cache is fresh.
    pub force: bool,
    /// File glob patterns to include (empty = all text files).
    pub include: Vec<String>,
    /// File glob patterns to exclude.
    pub exclude: Vec<String>,
    /// Maximum file size in bytes to index.  Default: 1 MiB.
    pub max_file_size: Option<u64>,
}

/// Handle returned immediately when indexing starts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHandle {
    /// Unique identifier for polling status.
    pub index_id: String,
    /// Path that is being indexed.
    pub path: String,
}

/// Status of an indexing operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IndexStatus {
    /// Queued, not yet started.
    Pending,
    /// Indexing in progress.
    Indexing {
        /// Progress fraction (0.0–1.0).
        progress: f32,
    },
    /// Indexing complete.
    Ready(IndexResult),
    /// Indexing failed.
    Failed(String),
}

/// Result of a completed indexing operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    /// Path that was indexed.
    pub path: String,
    /// Number of files indexed.
    pub files_indexed: usize,
    /// Number of chunks produced.
    pub chunks_produced: usize,
    /// Whether the index was already fresh (no work needed).
    pub was_cached: bool,
}

/// Options for `semantic_search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SemanticSearchOptions {
    /// Maximum number of results.  Default: 10.
    pub top_k: Option<usize>,
    /// Minimum similarity score (0.0–1.0).  Results below this are excluded.
    pub min_score: Option<f32>,
    /// Filter to files matching these glob patterns.
    pub file_filter: Vec<String>,
    /// Whether to rerank results with a cross-encoder.  Default: true.
    pub rerank: Option<bool>,
}

/// A single semantic search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    /// File path containing the match.
    pub file: String,
    /// Start line (1-indexed).
    pub line_start: usize,
    /// End line (1-indexed).
    pub line_end: usize,
    /// Matched content chunk.
    pub content: String,
    /// Similarity score (0.0–1.0).
    pub score: f32,
    /// Symbol name from AST-aware chunking (e.g. function name).
    pub symbol: Option<String>,
    /// Detected language.
    pub language: Option<String>,
}

/// Events emitted by the indexing pipeline for async notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IndexEvent {
    /// Indexing progress update.
    Progress {
        /// Index operation ID.
        index_id: String,
        /// Progress fraction (0.0–1.0).
        progress: f32,
    },
    /// Indexing completed successfully.
    Complete {
        /// Index operation ID.
        index_id: String,
        /// Result details.
        result: IndexResult,
    },
    /// Indexing failed.
    Failed {
        /// Index operation ID.
        index_id: String,
        /// Error description.
        error: String,
    },
    /// File watcher detected a change and re-indexed.
    FileChanged {
        /// Index operation ID.
        index_id: String,
        /// Changed file path.
        path: String,
    },
}

// ── Hybrid search ─────────────────────────────────────────────────────────────

/// Options for `hybrid_search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HybridSearchOptions {
    /// Weight applied to the BM25 score (`1.0` = pure BM25, `0.0` = pure vector).
    /// Default: `0.5`.
    pub alpha: f32,
    /// Maximum number of results to return.  Default: 10.
    pub top_k: usize,
}

impl Default for HybridSearchOptions {
    fn default() -> Self {
        Self {
            alpha: 0.5,
            top_k: 10,
        }
    }
}

/// A single result from a hybrid search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HybridSearchResult {
    /// Document identifier.
    pub id: String,
    /// Source file path.
    pub file: String,
    /// Optional symbol name (function, class, etc.).
    pub symbol: Option<String>,
    /// Matched content snippet.
    pub content: String,
    /// Combined relevance score.
    pub score: f32,
}

// ── Community detection types ─────────────────────────────────────────────────

/// Options for `community_search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunitySearchOptions {
    /// Maximum number of communities to return.  Default: 10.
    pub top_k: usize,
}

impl Default for CommunitySearchOptions {
    fn default() -> Self {
        Self { top_k: 10 }
    }
}

/// A single community listing entry — returned by `list_communities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunityEntry {
    /// Opaque community identifier (u64).
    pub community_id: u64,
    /// Number of member symbols in this community.
    pub member_count: usize,
}

/// Members of a community — returned by `community_members`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunityMembersResult {
    /// Opaque community identifier (u64).
    pub community_id: u64,
    /// Symbol names belonging to this community.
    pub members: Vec<String>,
}

/// Result of a community search — returned by `community_search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunitySearchResult {
    /// Opaque community identifier (u64).
    pub community_id: u64,
    /// Number of member symbols in this community.
    pub member_count: usize,
    /// Members whose names matched the query.
    pub matched_members: Vec<String>,
}

/// A community summary — returned by `community_summary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunitySummaryResult {
    /// Opaque community identifier (u64).
    pub community_id: u64,
    /// Generated or fallback summary text.
    pub summary: String,
    /// Whether the summary is considered stale.
    pub is_stale: bool,
}

// ── Sandbox types (kept here for shared use) ─────────────────────────────────

/// Shell execution response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    /// Exit code.
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

/// Process information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process ID.
    pub pid: u32,
    /// Command line.
    pub command: String,
    /// CPU usage percentage.
    pub cpu_pct: Option<f32>,
    /// Memory usage in bytes.
    pub mem_bytes: Option<u64>,
    /// Parent process ID.
    pub parent_pid: Option<u32>,
    /// Process state.
    pub state: String,
}

/// Background job information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    /// Job ID.
    pub id: String,
    /// Process ID.
    pub pid: Option<u32>,
    /// Command line.
    pub command: String,
    /// Job status.
    pub status: String,
}

/// Archive entry metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    /// Entry path within archive.
    pub path: String,
    /// Is directory.
    pub is_dir: bool,
    /// Uncompressed size.
    pub size: u64,
}

/// Archive information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveInfo {
    /// Archive entries.
    pub entries: Vec<ArchiveEntry>,
    /// Archive format (tar, zip, etc.).
    pub format: String,
    /// Compressed size in bytes.
    pub compressed_size: u64,
}

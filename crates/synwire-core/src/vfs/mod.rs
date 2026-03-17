//! Virtual filesystem abstraction for agent data operations.
//!
//! Provides a filesystem-like interface over heterogeneous data sources,
//! allowing LLMs to interact with any data source using familiar operations
//! (ls, read, write, cd, cp, mv, etc.).
//!
//! Operations mirror Linux coreutils with their most useful arguments
//! abstracted into option structs.  Providers declare which operations
//! they support via [`VfsCapabilities`].

#[cfg(feature = "agentic-ignore")]
pub mod agentic_ignore;
pub mod error;
pub mod grep_options;
pub mod memory;
pub mod output;
pub mod protocol;
#[allow(
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value,
    clippy::missing_const_for_fn
)]
pub mod tools;
pub mod types;

pub use error::VfsError;
pub use grep_options::{GrepOptions, GrepOutputMode};
pub use memory::MemoryProvider;
pub use output::{OutputFormat, format_output};
pub use protocol::Vfs;
pub use tools::vfs_tools;
pub use types::{
    CommunityEntry, CommunityMembersResult, CommunitySearchOptions, CommunitySearchResult,
    CommunitySummaryResult, CpOptions, DiffOptions, DiffResult, DirEntry, DiskUsage, DuOptions,
    EditResult, FileContent, FileInfo, FindEntry, FindOptions, GlobEntry, GrepMatch,
    HeadTailOptions, HybridSearchOptions, HybridSearchResult, IndexEvent, IndexHandle,
    IndexOptions, IndexResult, IndexStatus, LsOptions, MkdirOptions, MountInfo, ReadRange,
    RmOptions, SemanticSearchOptions, SemanticSearchResult, SortField, TransferResult, TreeEntry,
    TreeOptions, VfsCapabilities, WordCount, WriteResult,
};

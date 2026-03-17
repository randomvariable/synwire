//! Grep options and result types.

use serde::{Deserialize, Serialize};

/// Grep output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum GrepOutputMode {
    /// Return matching lines with content.
    #[default]
    Content,
    /// Return only file paths with matches.
    FilesWithMatches,
    /// Return match counts per file.
    Count,
}

/// Grep search options (ripgrep-style).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct GrepOptions {
    /// Search root path (None = current working directory).
    pub path: Option<String>,
    /// Lines of context after each match.
    pub after_context: u32,
    /// Lines of context before each match.
    pub before_context: u32,
    /// Symmetric context (overrides before/after if set).
    pub context: Option<u32>,
    /// Case-insensitive search.
    pub case_insensitive: bool,
    /// File glob filter (e.g., "*.rs").
    pub glob: Option<String>,
    /// File type filter (e.g., "rust", "python").
    pub file_type: Option<String>,
    /// Maximum number of matches to return.
    pub max_matches: Option<usize>,
    /// Output mode.
    pub output_mode: GrepOutputMode,
    /// Enable multiline matching (pattern can span lines).
    pub multiline: bool,
    /// Include line numbers in output.
    pub line_numbers: bool,
    /// Invert match (show non-matching lines).
    pub invert: bool,
    /// Treat pattern as literal string (not regex).
    pub fixed_string: bool,
}

//! Configuration for the semantic index pipeline.

use std::path::PathBuf;

/// Configuration for the semantic indexing pipeline.
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Override for the OS cache base directory.  Defaults to the platform default.
    pub cache_base: Option<PathBuf>,
    /// Target chunk size in characters.  Default: 1500.
    pub chunk_size: usize,
    /// Overlap between consecutive text chunks in characters.  Default: 200.
    pub chunk_overlap: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            cache_base: None,
            chunk_size: 1500,
            chunk_overlap: 200,
        }
    }
}

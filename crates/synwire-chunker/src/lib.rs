//! Tree-sitter AST-aware code chunking for Synwire semantic search.
//!
//! Splits source files into semantic chunks (functions, classes, structs)
//! using tree-sitter for code files, and a recursive character text splitter
//! for all other content.
//!
//! # Quick start
//!
//! ```
//! use synwire_chunker::{Chunker, ChunkOptions};
//!
//! let chunker = Chunker::new();
//! let docs = chunker.chunk_file(
//!     "src/main.rs",
//!     "pub fn greet(name: &str) -> String { format!(\"Hello, {name}!\") }",
//! );
//! assert!(!docs.is_empty());
//! ```

#![forbid(unsafe_code)]

mod ast_chunker;
mod chunker;
mod language;
mod text_chunker;

pub use chunker::{ChunkOptions, Chunker};
pub use language::{Language, detect_language};

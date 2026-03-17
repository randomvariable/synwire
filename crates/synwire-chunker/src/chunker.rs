//! Main [`Chunker`] entry point.
//!
//! Combines AST-aware chunking for recognised source languages with a
//! recursive character text splitter for everything else.

use std::path::Path;

use synwire_core::documents::Document;

use crate::ast_chunker::chunk_ast;
use crate::language::detect_language;
use crate::text_chunker::chunk_text;

/// Options controlling how the [`Chunker`] splits content.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChunkOptions {
    /// Target chunk size in bytes for the text-splitter fallback.
    ///
    /// Defaults to `1500`.
    pub chunk_size: usize,
    /// Number of bytes of overlap between consecutive text chunks.
    ///
    /// Defaults to `200`.
    pub overlap: usize,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            chunk_size: 1500,
            overlap: 200,
        }
    }
}

/// Splits source files into semantic [`Document`] chunks for embedding.
///
/// Uses tree-sitter AST chunking for recognised languages (Rust, Python, Go,
/// etc.) and falls back to a recursive character text splitter for all other
/// content.
///
/// # Examples
///
/// ```
/// use synwire_chunker::Chunker;
///
/// let chunker = Chunker::new();
/// let docs = chunker.chunk_file("src/main.rs", "fn main() {}");
/// assert!(!docs.is_empty());
/// ```
#[derive(Debug, Clone, Default)]
pub struct Chunker {
    /// Options controlling chunk size and overlap for the text-splitter path.
    pub options: ChunkOptions,
}

impl Chunker {
    /// Create a new [`Chunker`] with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new [`Chunker`] with the provided [`ChunkOptions`].
    pub const fn with_options(options: ChunkOptions) -> Self {
        Self { options }
    }

    /// Chunk `content` from `file_path` into [`Document`]s.
    ///
    /// 1. Detects the language from the file extension.
    /// 2. Attempts AST chunking via tree-sitter.
    /// 3. Falls back to the recursive character text splitter if the language
    ///    is unrecognised, has no compatible grammar, or no top-level
    ///    definitions are found.
    pub fn chunk_file(&self, file_path: &str, content: &str) -> Vec<Document> {
        let path = Path::new(file_path);
        if let Some(lang) = detect_language(path) {
            let chunks = chunk_ast(file_path, content, lang);
            if !chunks.is_empty() {
                return chunks;
            }
        }
        chunk_text(
            file_path,
            content,
            self.options.chunk_size,
            self.options.overlap,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_file_uses_ast_chunking() {
        let chunker = Chunker::new();
        let src = "pub fn hello() -> &'static str { \"hello\" }\npub fn world() {}";
        let docs = chunker.chunk_file("lib.rs", src);
        // AST chunking should find at least one function.
        assert!(!docs.is_empty());
        let has_symbol = docs.iter().any(|d| d.metadata.contains_key("symbol"));
        assert!(has_symbol, "expected AST chunks to have 'symbol' metadata");
    }

    #[test]
    fn unknown_extension_falls_back_to_text() {
        let chunker = Chunker::new();
        let docs = chunker.chunk_file("data.bin", "some binary-ish content here");
        assert!(!docs.is_empty());
        // Text chunker sets chunk_index, not symbol.
        assert!(docs[0].metadata.contains_key("chunk_index"));
    }

    #[test]
    fn empty_file_returns_no_chunks() {
        let chunker = Chunker::new();
        assert!(chunker.chunk_file("empty.txt", "").is_empty());
    }

    #[test]
    fn custom_options_respected() {
        let opts = ChunkOptions {
            chunk_size: 50,
            overlap: 5,
        };
        let chunker = Chunker::with_options(opts);
        let content = "word ".repeat(100);
        // Unknown extension triggers text fallback with our small chunk size.
        let docs = chunker.chunk_file("notes.bin", &content);
        assert!(
            docs.len() > 1,
            "expected multiple chunks with chunk_size=50"
        );
    }
}

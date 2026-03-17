//! Property tests: structural invariants of the `Chunker`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_chunker::{ChunkOptions, Chunker};

fn make_chunker(chunk_size: usize, overlap: usize) -> Chunker {
    let mut opts = ChunkOptions::default();
    opts.chunk_size = chunk_size;
    opts.overlap = overlap;
    Chunker::with_options(opts)
}

proptest! {
    /// Non-empty source text always produces at least one chunk, regardless
    /// of chunk size and overlap.
    #[test]
    fn non_empty_source_produces_chunks(
        content in ".{1,2000}",
        chunk_size in 50_usize..=2000,
        overlap in 0_usize..=100,
    ) {
        let chunker = make_chunker(chunk_size, overlap);
        // Unknown extension forces the text-splitter path.
        let docs = chunker.chunk_file("data.bin", &content);
        prop_assert!(!docs.is_empty(), "expected at least one chunk for non-empty input");
    }

    /// Every chunk's `page_content` must be non-empty.
    #[test]
    fn all_chunks_have_non_empty_content(
        content in ".{1,2000}",
        chunk_size in 50_usize..=2000,
        overlap in 0_usize..=100,
    ) {
        let chunker = make_chunker(chunk_size, overlap);
        let docs = chunker.chunk_file("data.bin", &content);
        for doc in &docs {
            prop_assert!(!doc.page_content.is_empty(), "chunk page_content must not be empty");
        }
    }

    /// Every text-splitter chunk must be a substring of the source text.
    #[test]
    fn all_chunks_are_substrings_of_source(
        content in "[a-zA-Z0-9 \n]{1,2000}",
        chunk_size in 50_usize..=2000,
        overlap in 0_usize..=100,
    ) {
        let chunker = make_chunker(chunk_size, overlap);
        let docs = chunker.chunk_file("data.bin", &content);
        for doc in &docs {
            prop_assert!(
                content.contains(doc.page_content.as_str()),
                "chunk '{}' is not a substring of source",
                &doc.page_content[..doc.page_content.len().min(40)]
            );
        }
    }

    /// Empty source text always produces zero chunks.
    #[test]
    fn empty_source_produces_no_chunks(
        chunk_size in 50_usize..=2000,
        overlap in 0_usize..=100,
    ) {
        let chunker = make_chunker(chunk_size, overlap);
        let docs = chunker.chunk_file("data.bin", "");
        prop_assert!(docs.is_empty(), "expected zero chunks for empty input");
    }
}

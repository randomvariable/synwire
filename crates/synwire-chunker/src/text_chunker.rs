//! Fallback text splitter for non-code content.
//!
//! Implements a recursive character splitter that tries progressively finer
//! split points (`\n\n`, `\n`, ` `, then individual characters) to keep
//! chunks near a target size while preserving contextual overlap.

use std::collections::HashMap;

use serde_json::Value;
use synwire_core::documents::Document;

/// Separators tried in order, from coarsest to finest granularity.
const SEPARATORS: &[&str] = &["\n\n", "\n", " ", ""];

/// Split `text` at the first separator that produces a prefix no longer than
/// `max_len` bytes, returning `(head, rest)`.
///
/// If no separator fits within `max_len`, the text is split at the character
/// boundary `max_len` bytes in (using the empty-string separator as fallback).
fn split_at_separator(text: &str, max_len: usize) -> (&str, &str) {
    // Try each separator from coarsest to finest.
    for sep in SEPARATORS {
        if sep.is_empty() {
            // Character-boundary split: find the last char boundary <= max_len.
            let split_point = text
                .char_indices()
                .take_while(|(i, _)| *i < max_len)
                .last()
                .map_or_else(|| text.len().min(max_len), |(i, c)| i + c.len_utf8());
            return text.split_at(split_point);
        }

        // Find the last occurrence of `sep` that keeps the prefix within max_len.
        // Walk back to a char boundary so the slice doesn't panic on multi-byte chars.
        let safe_end = {
            let mut b = text.len().min(max_len);
            while b > 0 && !text.is_char_boundary(b) {
                b -= 1;
            }
            b
        };
        if let Some(pos) = text[..safe_end].rfind(sep) {
            let split_point = pos + sep.len();
            if split_point > 0 && split_point <= text.len() {
                return text.split_at(split_point);
            }
        }
    }

    // Unreachable because the empty-string separator always handles the split,
    // but we return the whole text as a safe fallback.
    (text, "")
}

/// Compute the 1-indexed line number of the byte at `byte_offset` within
/// `content`.
fn line_number_at(content: &str, byte_offset: usize) -> usize {
    let safe_offset = byte_offset.min(content.len());
    content[..safe_offset]
        .chars()
        .filter(|&c| c == '\n')
        .count()
        + 1
}

/// Split text into overlapping chunks using a recursive character splitter.
///
/// Tries to split at paragraph boundaries (`\n\n`), then newlines, then
/// spaces, then characters — always keeping chunks near `chunk_size` bytes
/// with `overlap` bytes of context between consecutive chunks.
///
/// Each returned [`Document`] carries metadata keys:
/// - `"file"` — the `file_path` argument,
/// - `"chunk_index"` — 0-based chunk position,
/// - `"line_start"` — 1-indexed first line of the chunk,
/// - `"line_end"` — 1-indexed last line of the chunk.
///
/// # Panics
///
/// This function does not panic.
pub fn chunk_text(
    file_path: &str,
    content: &str,
    chunk_size: usize,
    overlap: usize,
) -> Vec<Document> {
    if content.is_empty() {
        return Vec::new();
    }

    // Guard against degenerate settings.
    let effective_chunk = chunk_size.max(1);
    let effective_overlap = overlap.min(effective_chunk.saturating_sub(1));

    let mut docs: Vec<Document> = Vec::new();
    let mut byte_start: usize = 0;
    let mut chunk_index: usize = 0;

    while byte_start < content.len() {
        let remaining = &content[byte_start..];

        // If all remaining text fits within one chunk, take it whole.
        let chunk_bytes: &str = if remaining.len() <= effective_chunk {
            remaining
        } else {
            let (head, _) = split_at_separator(remaining, effective_chunk);
            if head.is_empty() { remaining } else { head }
        };

        if !chunk_bytes.is_empty() {
            let abs_start = byte_start;
            let abs_end = byte_start + chunk_bytes.len();

            let line_start = line_number_at(content, abs_start);
            let line_end = line_number_at(content, abs_end.saturating_sub(1));

            let mut metadata: HashMap<String, Value> = HashMap::new();
            let _ = metadata.insert("file".to_owned(), Value::String(file_path.to_owned()));
            let _ = metadata.insert("chunk_index".to_owned(), Value::Number(chunk_index.into()));
            let _ = metadata.insert("line_start".to_owned(), Value::Number(line_start.into()));
            let _ = metadata.insert("line_end".to_owned(), Value::Number(line_end.into()));

            docs.push(Document::with_metadata(chunk_bytes.to_owned(), metadata));
            chunk_index += 1;
        }

        // Advance past this chunk, stepping back by `overlap` to provide context.
        let advance = chunk_bytes.len().saturating_sub(effective_overlap);
        if advance == 0 {
            // Prevent infinite loop if overlap >= chunk length.
            break;
        }
        byte_start += advance;
    }

    docs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_content_returns_no_chunks() {
        assert!(chunk_text("file.txt", "", 500, 50).is_empty());
    }

    #[test]
    fn small_content_fits_in_one_chunk() {
        let content = "Hello, world!";
        let chunks = chunk_text("file.txt", content, 500, 50);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].page_content, content);
    }

    #[test]
    fn chunks_larger_content() {
        let content = "line1\nline2\nline3\nline4\nline5\n".repeat(20);
        let chunks = chunk_text("file.txt", &content, 100, 20);
        assert!(chunks.len() > 1, "expected multiple chunks");
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn chunk_index_is_sequential() {
        let content = "a".repeat(1000);
        let chunks = chunk_text("file.txt", &content, 100, 10);
        for (i, doc) in chunks.iter().enumerate() {
            let idx = doc
                .metadata
                .get("chunk_index")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(u64::MAX);
            assert_eq!(idx as usize, i, "chunk_index out of order at position {i}");
        }
    }

    #[test]
    fn line_numbers_are_one_indexed() {
        let content = "line1\nline2\nline3\n";
        let chunks = chunk_text("file.txt", content, 500, 0);
        let line_start = chunks[0]
            .metadata
            .get("line_start")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        assert_eq!(line_start, 1);
    }

    #[test]
    fn file_metadata_is_set() {
        let chunks = chunk_text("docs/readme.md", "some content", 500, 0);
        assert_eq!(
            chunks[0].metadata.get("file").and_then(|v| v.as_str()),
            Some("docs/readme.md")
        );
    }
}

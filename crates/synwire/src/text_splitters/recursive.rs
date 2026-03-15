//! Recursive character text splitter.

use super::character::merge_splits;

/// Splits text using a hierarchy of separators, trying larger separators first.
///
/// The default separator hierarchy is `["\n\n", "\n", " ", ""]`, which
/// attempts to split on paragraph boundaries first, then line boundaries,
/// then word boundaries, and finally character boundaries.
///
/// # Examples
///
/// ```
/// use synwire::text_splitters::RecursiveCharacterTextSplitter;
///
/// let splitter = RecursiveCharacterTextSplitter::new(50, 10);
/// let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
/// let chunks = splitter.split_text(text);
/// assert!(!chunks.is_empty());
/// ```
pub struct RecursiveCharacterTextSplitter {
    chunk_size: usize,
    chunk_overlap: usize,
    separators: Vec<String>,
}

impl RecursiveCharacterTextSplitter {
    /// Creates a new recursive splitter with default separators.
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            separators: vec![
                "\n\n".to_owned(),
                "\n".to_owned(),
                " ".to_owned(),
                String::new(),
            ],
        }
    }

    /// Creates a splitter with custom separators.
    pub const fn with_separators(
        chunk_size: usize,
        chunk_overlap: usize,
        separators: Vec<String>,
    ) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            separators,
        }
    }

    /// Splits the input text into chunks.
    pub fn split_text(&self, text: &str) -> Vec<String> {
        self.split_recursive(text, &self.separators)
    }

    /// Recursively splits text, trying each separator in order.
    fn split_recursive(&self, text: &str, separators: &[String]) -> Vec<String> {
        if text.is_empty() {
            return Vec::new();
        }

        if text.len() <= self.chunk_size {
            return vec![text.to_owned()];
        }

        // Find the appropriate separator
        let (separator, remaining_seps) = Self::find_separator(text, separators);

        let sep_str: &str = separator;
        let pieces: Vec<&str> = if sep_str.is_empty() {
            text.char_indices()
                .map(|(i, c)| &text[i..i + c.len_utf8()])
                .collect()
        } else {
            text.split(sep_str).collect()
        };

        let mut final_chunks = Vec::new();
        let mut good_pieces: Vec<&str> = Vec::new();

        for &piece in &pieces {
            if piece.len() <= self.chunk_size {
                good_pieces.push(piece);
            } else {
                // Flush accumulated good pieces
                if !good_pieces.is_empty() {
                    let merged =
                        merge_splits(&good_pieces, sep_str, self.chunk_size, self.chunk_overlap);
                    final_chunks.extend(merged);
                    good_pieces.clear();
                }
                // Recursively split the oversized piece
                let sub_chunks = self.split_recursive(piece, remaining_seps);
                final_chunks.extend(sub_chunks);
            }
        }

        // Flush remaining good pieces
        if !good_pieces.is_empty() {
            let merged = merge_splits(&good_pieces, sep_str, self.chunk_size, self.chunk_overlap);
            final_chunks.extend(merged);
        }

        final_chunks
    }

    /// Finds the most appropriate separator for the text.
    fn find_separator<'a>(text: &str, separators: &'a [String]) -> (&'a str, &'a [String]) {
        for (i, sep) in separators.iter().enumerate() {
            if sep.is_empty() || text.contains(sep.as_str()) {
                let remaining = if i + 1 < separators.len() {
                    &separators[i + 1..]
                } else {
                    &[]
                };
                return (sep, remaining);
            }
        }
        // Fallback to last separator
        separators.last().map_or(("", &[]), |last| (last, &[]))
    }
}

impl Default for RecursiveCharacterTextSplitter {
    fn default() -> Self {
        Self::new(1000, 200)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_paragraph_boundaries_first() {
        let splitter = RecursiveCharacterTextSplitter::new(25, 0);
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = splitter.split_text(text);
        // Each paragraph is 16-17 chars, under 25. Two paragraphs + \n\n = 36, over 25.
        // So each paragraph should be its own chunk.
        assert!(
            chunks.len() >= 2,
            "expected >= 2 chunks, got {}: {chunks:?}",
            chunks.len()
        );
        for chunk in &chunks {
            assert!(
                chunk.len() <= 25,
                "chunk too long: '{chunk}' (len={})",
                chunk.len()
            );
        }
    }

    #[test]
    fn falls_back_to_line_separator() {
        let splitter = RecursiveCharacterTextSplitter::new(30, 0);
        let text = "Line one\nLine two\nLine three\nLine four";
        let chunks = splitter.split_text(text);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn falls_back_to_space_separator() {
        let splitter = RecursiveCharacterTextSplitter::new(10, 0);
        let text = "one two three four five";
        let chunks = splitter.split_text(text);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.len() <= 10, "chunk '{chunk}' exceeds limit");
        }
    }

    #[test]
    fn empty_text_returns_empty() {
        let splitter = RecursiveCharacterTextSplitter::new(10, 0);
        let chunks = splitter.split_text("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn text_under_limit_returns_single_chunk() {
        let splitter = RecursiveCharacterTextSplitter::new(100, 0);
        let chunks = splitter.split_text("Short text.");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Short text.");
    }

    #[test]
    fn custom_separators() {
        let splitter = RecursiveCharacterTextSplitter::with_separators(
            20,
            0,
            vec!["---".to_owned(), " ".to_owned()],
        );
        let text = "Part A---Part B---Part C";
        let chunks = splitter.split_text(text);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn respects_chunk_overlap() {
        let splitter = RecursiveCharacterTextSplitter::new(20, 5);
        let text = "aaaa\n\nbbbb\n\ncccc\n\ndddd";
        let chunks = splitter.split_text(text);
        assert!(chunks.len() >= 2);
    }
}

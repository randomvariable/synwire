//! Character-based text splitter.

/// Splits text into chunks by a single separator string.
///
/// Chunks are created by splitting on the separator, then merging
/// consecutive pieces until `chunk_size` is reached. Adjacent chunks
/// overlap by `chunk_overlap` characters.
///
/// # Examples
///
/// ```
/// use synwire::text_splitters::CharacterTextSplitter;
///
/// let splitter = CharacterTextSplitter::new(20, 5, "\n");
/// let chunks = splitter.split_text("Hello\nWorld\nFoo\nBar");
/// assert!(!chunks.is_empty());
/// ```
pub struct CharacterTextSplitter {
    chunk_size: usize,
    chunk_overlap: usize,
    separator: String,
}

impl CharacterTextSplitter {
    /// Creates a new character text splitter.
    pub fn new(chunk_size: usize, chunk_overlap: usize, separator: &str) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            separator: separator.to_owned(),
        }
    }

    /// Splits the input text into chunks.
    pub fn split_text(&self, text: &str) -> Vec<String> {
        let pieces: Vec<&str> = if self.separator.is_empty() {
            // Split by character
            text.char_indices()
                .map(|(i, c)| &text[i..i + c.len_utf8()])
                .collect()
        } else {
            text.split(&self.separator).collect()
        };

        merge_splits(
            &pieces,
            &self.separator,
            self.chunk_size,
            self.chunk_overlap,
        )
    }
}

impl Default for CharacterTextSplitter {
    fn default() -> Self {
        Self::new(1000, 200, "\n\n")
    }
}

/// Merges split pieces into chunks respecting size and overlap constraints.
pub fn merge_splits(
    pieces: &[&str],
    separator: &str,
    chunk_size: usize,
    chunk_overlap: usize,
) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_parts: Vec<&str> = Vec::new();
    let mut current_len: usize = 0;

    for &piece in pieces {
        let piece_len = piece.len();
        let sep_len = if current_parts.is_empty() {
            0
        } else {
            separator.len()
        };

        if current_len + sep_len + piece_len > chunk_size && !current_parts.is_empty() {
            let chunk = current_parts.join(separator);
            if !chunk.is_empty() {
                chunks.push(chunk);
            }

            // Remove leading parts to achieve overlap
            while current_len > chunk_overlap && !current_parts.is_empty() {
                if current_parts.len() == 1 && chunk_overlap > 0 {
                    break;
                }
                let removed = current_parts.remove(0);
                current_len = current_len.saturating_sub(removed.len()).saturating_sub(
                    if current_parts.is_empty() {
                        0
                    } else {
                        separator.len()
                    },
                );
            }
        }

        current_parts.push(piece);
        current_len += if current_parts.len() == 1 {
            piece_len
        } else {
            separator.len() + piece_len
        };
    }

    if !current_parts.is_empty() {
        let chunk = current_parts.join(separator);
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
    }

    chunks
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn split_respects_chunk_size() {
        let splitter = CharacterTextSplitter::new(10, 0, " ");
        let chunks = splitter.split_text("aa bb cc dd ee ff");
        for chunk in &chunks {
            assert!(
                chunk.len() <= 10,
                "chunk too long: '{chunk}' (len={})",
                chunk.len()
            );
        }
    }

    #[test]
    fn split_with_overlap() {
        let splitter = CharacterTextSplitter::new(10, 3, " ");
        let chunks = splitter.split_text("aa bb cc dd ee");
        // With overlap, later chunks should start before the previous chunk ended
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn empty_text_returns_empty() {
        let splitter = CharacterTextSplitter::new(10, 0, "\n");
        let chunks = splitter.split_text("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn single_piece_under_limit() {
        let splitter = CharacterTextSplitter::new(100, 0, "\n");
        let chunks = splitter.split_text("Hello World");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello World");
    }

    #[test]
    fn newline_separator() {
        let splitter = CharacterTextSplitter::new(15, 0, "\n");
        let chunks = splitter.split_text("Hello\nWorld\nFoo\nBar");
        assert!(chunks.len() >= 2);
    }
}

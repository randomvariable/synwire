//! Cursor-based pagination helper with a 1000-page safeguard cap.

/// Maximum number of pages that will be fetched in a single paginated request.
///
/// This cap prevents runaway loops when an MCP server returns infinite cursors.
pub const MAX_PAGES: usize = 1_000;

/// Tracks the state of a cursor-based pagination sequence.
///
/// Call [`advance`](Self::advance) after each page to advance the cursor.
/// The helper enforces [`MAX_PAGES`] as a hard cap.
#[derive(Debug, Clone)]
pub struct PaginationCursor {
    /// The opaque cursor string returned by the server, or `None` for the first page.
    cursor: Option<String>,
    /// Number of pages fetched so far.
    pages_fetched: usize,
}

impl PaginationCursor {
    /// Creates a new pagination cursor at the start of a sequence.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cursor: None,
            pages_fetched: 0,
        }
    }

    /// Returns the current cursor value to include in the next list request.
    ///
    /// `None` indicates the first page (no cursor needed).
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    /// Returns `true` if more pages may be available.
    ///
    /// Returns `false` when the page cap ([`MAX_PAGES`]) has been reached.
    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.pages_fetched < MAX_PAGES
    }

    /// Returns the number of pages fetched so far.
    #[must_use]
    pub const fn pages_fetched(&self) -> usize {
        self.pages_fetched
    }

    /// Advances the cursor after a page has been received.
    ///
    /// Pass `Some(cursor)` when the server returns a next-page cursor,
    /// or `None` when the last page has been reached.
    ///
    /// Returns `true` if there are more pages to fetch, `false` otherwise.
    #[must_use]
    pub fn advance(&mut self, next_cursor: Option<String>) -> bool {
        self.pages_fetched += 1;
        self.cursor = next_cursor;
        self.cursor.is_some() && self.has_more()
    }
}

impl Default for PaginationCursor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_no_cursor() {
        let cursor = PaginationCursor::new();
        assert!(cursor.current().is_none());
        assert!(cursor.has_more());
        assert_eq!(cursor.pages_fetched(), 0);
    }

    #[test]
    fn advances_through_pages() {
        let mut cursor = PaginationCursor::new();
        assert!(cursor.advance(Some("page2".into())));
        assert_eq!(cursor.current(), Some("page2"));
        assert_eq!(cursor.pages_fetched(), 1);

        assert!(!cursor.advance(None));
        assert!(cursor.current().is_none());
        assert_eq!(cursor.pages_fetched(), 2);
    }

    #[test]
    fn enforces_max_pages_cap() {
        let mut cursor = PaginationCursor::new();
        for i in 0..MAX_PAGES {
            let has_more = cursor.advance(Some(format!("page{i}")));
            if i == MAX_PAGES - 1 {
                // At cap: has_more should be false even though cursor is Some
                assert!(!has_more, "should stop at page cap");
            }
        }
        assert!(!cursor.has_more());
    }
}

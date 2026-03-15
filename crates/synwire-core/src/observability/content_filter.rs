//! Trace content filter for controlling what is captured in spans.

use serde::{Deserialize, Serialize};

/// Controls which content is included in observability traces.
///
/// By default, all content categories are included with no length limit.
/// Use this struct to redact sensitive information or reduce trace payload
/// sizes.
///
/// # Example
///
/// ```
/// use synwire_core::observability::TraceContentFilter;
///
/// let filter = TraceContentFilter::builder()
///     .include_system_instructions(false)
///     .max_content_length(Some(512))
///     .build();
///
/// assert!(!filter.include_system_instructions);
/// assert_eq!(filter.max_content_length, Some(512));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct TraceContentFilter {
    /// Whether to include input messages in traces.
    pub include_input_messages: bool,
    /// Whether to include output messages in traces.
    pub include_output_messages: bool,
    /// Whether to include system instructions in traces.
    pub include_system_instructions: bool,
    /// Whether to include tool arguments in traces.
    pub include_tool_arguments: bool,
    /// Whether to include tool results in traces.
    pub include_tool_results: bool,
    /// Whether to include retrieval queries in traces.
    pub include_retrieval_queries: bool,
    /// Maximum content length per field (truncated if exceeded). `None` means
    /// no limit.
    pub max_content_length: Option<usize>,
}

impl Default for TraceContentFilter {
    fn default() -> Self {
        Self {
            include_input_messages: true,
            include_output_messages: true,
            include_system_instructions: true,
            include_tool_arguments: true,
            include_tool_results: true,
            include_retrieval_queries: true,
            max_content_length: None,
        }
    }
}

impl TraceContentFilter {
    /// Creates a builder for constructing a `TraceContentFilter`.
    pub fn builder() -> TraceContentFilterBuilder {
        TraceContentFilterBuilder::default()
    }

    /// Truncates the given string to [`max_content_length`](Self::max_content_length) if set.
    pub fn truncate<'a>(&self, content: &'a str) -> &'a str {
        match self.max_content_length {
            Some(max) if content.len() > max => {
                // Find a valid UTF-8 boundary at or before max.
                let end = content
                    .char_indices()
                    .take_while(|&(i, _)| i <= max)
                    .last()
                    .map_or(0, |(i, _)| i);
                &content[..end]
            }
            _ => content,
        }
    }
}

/// Builder for [`TraceContentFilter`].
#[derive(Debug, Default)]
pub struct TraceContentFilterBuilder {
    filter: TraceContentFilter,
}

impl TraceContentFilterBuilder {
    /// Sets whether to include input messages.
    pub const fn include_input_messages(mut self, value: bool) -> Self {
        self.filter.include_input_messages = value;
        self
    }

    /// Sets whether to include output messages.
    pub const fn include_output_messages(mut self, value: bool) -> Self {
        self.filter.include_output_messages = value;
        self
    }

    /// Sets whether to include system instructions.
    pub const fn include_system_instructions(mut self, value: bool) -> Self {
        self.filter.include_system_instructions = value;
        self
    }

    /// Sets whether to include tool arguments.
    pub const fn include_tool_arguments(mut self, value: bool) -> Self {
        self.filter.include_tool_arguments = value;
        self
    }

    /// Sets whether to include tool results.
    pub const fn include_tool_results(mut self, value: bool) -> Self {
        self.filter.include_tool_results = value;
        self
    }

    /// Sets whether to include retrieval queries.
    pub const fn include_retrieval_queries(mut self, value: bool) -> Self {
        self.filter.include_retrieval_queries = value;
        self
    }

    /// Sets the maximum content length.
    pub const fn max_content_length(mut self, value: Option<usize>) -> Self {
        self.filter.max_content_length = value;
        self
    }

    /// Builds the [`TraceContentFilter`].
    pub const fn build(self) -> TraceContentFilter {
        self.filter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_includes_all() {
        let filter = TraceContentFilter::default();
        assert!(filter.include_input_messages);
        assert!(filter.include_output_messages);
        assert!(filter.include_system_instructions);
        assert!(filter.include_tool_arguments);
        assert!(filter.include_tool_results);
        assert!(filter.include_retrieval_queries);
        assert!(filter.max_content_length.is_none());
    }

    #[test]
    fn builder_overrides() {
        let filter = TraceContentFilter::builder()
            .include_input_messages(false)
            .include_tool_results(false)
            .max_content_length(Some(100))
            .build();

        assert!(!filter.include_input_messages);
        assert!(!filter.include_tool_results);
        assert_eq!(filter.max_content_length, Some(100));
        // Others remain default
        assert!(filter.include_output_messages);
    }

    #[test]
    fn truncate_within_limit() {
        let filter = TraceContentFilter::builder()
            .max_content_length(Some(100))
            .build();
        let content = "short string";
        assert_eq!(filter.truncate(content), content);
    }

    #[test]
    fn truncate_exceeds_limit() {
        let filter = TraceContentFilter::builder()
            .max_content_length(Some(5))
            .build();
        let content = "hello world";
        let truncated = filter.truncate(content);
        assert!(truncated.len() <= 5);
    }

    #[test]
    fn truncate_no_limit() {
        let filter = TraceContentFilter::default();
        let content = "a".repeat(10_000);
        assert_eq!(filter.truncate(&content), content);
    }

    #[test]
    fn truncate_multibyte_boundary() {
        let filter = TraceContentFilter::builder()
            .max_content_length(Some(4))
            .build();
        // Unicode snowman is 3 bytes; two snowmen = 6 bytes
        let content = "\u{2603}\u{2603}";
        let truncated = filter.truncate(content);
        // Should truncate to one snowman (3 bytes) since second starts at 3
        assert_eq!(truncated, "\u{2603}");
    }
}

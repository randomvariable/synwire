//! Directive filtering and transformation.

use crate::agents::directive::Directive;

/// Filter decision for a directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilterDecision {
    /// Pass directive through unchanged.
    Pass,
    /// Suppress directive (don't execute).
    Suppress,
    /// Reject directive with error.
    Reject,
}

/// Filters and potentially transforms directives.
///
/// Filters can suppress, modify, or reject directives before execution.
pub trait DirectiveFilter: Send + Sync {
    /// Filter a directive.
    ///
    /// Returns `None` to suppress the directive, or `Some(modified)` to pass it through
    /// (potentially modified).
    fn filter(&self, directive: Directive) -> Option<Directive>;

    /// Get filter decision without consuming the directive.
    fn decision(&self, directive: &Directive) -> FilterDecision {
        match self.filter(directive.clone()) {
            Some(_) => FilterDecision::Pass,
            None => FilterDecision::Suppress,
        }
    }
}

/// Chain of filters applied in order.
#[derive(Default)]
pub struct FilterChain {
    filters: Vec<Box<dyn DirectiveFilter>>,
}

impl std::fmt::Debug for FilterChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterChain")
            .field("filters_count", &self.filters.len())
            .finish()
    }
}

impl FilterChain {
    /// Create a new empty filter chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a filter to the chain.
    pub fn add(&mut self, filter: Box<dyn DirectiveFilter>) {
        self.filters.push(filter);
    }

    /// Apply all filters in order.
    ///
    /// Returns `None` if any filter suppresses the directive.
    #[must_use]
    pub fn apply(&self, mut directive: Directive) -> Option<Directive> {
        for filter in &self.filters {
            directive = filter.filter(directive)?;
        }
        Some(directive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::directive::Directive;

    struct SuppressStopFilter;
    impl DirectiveFilter for SuppressStopFilter {
        fn filter(&self, directive: Directive) -> Option<Directive> {
            match directive {
                Directive::Stop { .. } => None,
                other => Some(other),
            }
        }
    }

    struct PassThroughFilter;
    impl DirectiveFilter for PassThroughFilter {
        fn filter(&self, directive: Directive) -> Option<Directive> {
            Some(directive)
        }
    }

    #[test]
    fn test_filter_chain_pass() {
        let mut chain = FilterChain::new();
        chain.add(Box::new(PassThroughFilter));

        let directive = Directive::Stop { reason: None };
        let result = chain.apply(directive);
        assert!(result.is_some());
    }

    #[test]
    fn test_filter_chain_suppress() {
        let mut chain = FilterChain::new();
        chain.add(Box::new(SuppressStopFilter));

        let directive = Directive::Stop { reason: None };
        let result = chain.apply(directive);
        assert!(result.is_none());
    }

    #[test]
    fn test_filter_chain_multiple_filters() {
        let mut chain = FilterChain::new();
        chain.add(Box::new(PassThroughFilter));
        chain.add(Box::new(SuppressStopFilter));

        let directive = Directive::Stop { reason: None };
        let result = chain.apply(directive);
        assert!(result.is_none()); // Suppressed by second filter
    }

    #[test]
    fn test_filter_decision() {
        let filter = SuppressStopFilter;
        let stop_directive = Directive::Stop { reason: None };
        let spawn_directive = Directive::SpawnTask {
            description: "test".to_string(),
            input: serde_json::json!({}),
        };

        assert_eq!(filter.decision(&stop_directive), FilterDecision::Suppress);
        assert_eq!(filter.decision(&spawn_directive), FilterDecision::Pass);
    }
}

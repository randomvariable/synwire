//! Metadata filtering for vector store queries.

use serde_json::Value;
use std::collections::HashMap;

/// A filter predicate for document metadata.
///
/// Supports equality, comparison, set membership, and boolean composition.
/// Used to narrow vector store search results beyond pure similarity.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MetadataFilter {
    /// Metadata field equals the given value.
    Eq(String, Value),
    /// Metadata field does not equal the given value.
    Ne(String, Value),
    /// Metadata field is greater than the given value.
    Gt(String, Value),
    /// Metadata field is less than the given value.
    Lt(String, Value),
    /// Metadata field is greater than or equal to the given value.
    Gte(String, Value),
    /// Metadata field is less than or equal to the given value.
    Lte(String, Value),
    /// Metadata field value is in the given set.
    In(String, Vec<Value>),
    /// All sub-filters must match.
    And(Vec<Self>),
    /// At least one sub-filter must match.
    Or(Vec<Self>),
}

impl MetadataFilter {
    /// Returns `true` if this filter matches the given metadata.
    pub fn matches(&self, metadata: &HashMap<String, Value>) -> bool {
        match self {
            Self::Eq(key, value) => metadata.get(key).is_some_and(|v| v == value),
            Self::Ne(key, value) => metadata.get(key).is_none_or(|v| v != value),
            Self::Gt(key, value) => metadata
                .get(key)
                .is_some_and(|v| compare_values(v, value) == Some(std::cmp::Ordering::Greater)),
            Self::Lt(key, value) => metadata
                .get(key)
                .is_some_and(|v| compare_values(v, value) == Some(std::cmp::Ordering::Less)),
            Self::Gte(key, value) => metadata.get(key).is_some_and(|v| {
                matches!(
                    compare_values(v, value),
                    Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                )
            }),
            Self::Lte(key, value) => metadata.get(key).is_some_and(|v| {
                matches!(
                    compare_values(v, value),
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                )
            }),
            Self::In(key, values) => metadata.get(key).is_some_and(|v| values.contains(v)),
            Self::And(filters) => filters.iter().all(|f| f.matches(metadata)),
            Self::Or(filters) => filters.iter().any(|f| f.matches(metadata)),
        }
    }
}

/// Compares two JSON values numerically if both are numbers.
fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a.as_f64(), b.as_f64()) {
        (Some(av), Some(bv)) => av.partial_cmp(&bv),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_metadata(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn filter_eq_matches() {
        let meta = make_metadata(&[("color", json!("red"))]);
        let filter = MetadataFilter::Eq("color".into(), json!("red"));
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_eq_does_not_match() {
        let meta = make_metadata(&[("color", json!("blue"))]);
        let filter = MetadataFilter::Eq("color".into(), json!("red"));
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn filter_ne_matches() {
        let meta = make_metadata(&[("color", json!("blue"))]);
        let filter = MetadataFilter::Ne("color".into(), json!("red"));
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_ne_missing_key_matches() {
        let meta = make_metadata(&[]);
        let filter = MetadataFilter::Ne("color".into(), json!("red"));
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_in_matches() {
        let meta = make_metadata(&[("status", json!("active"))]);
        let filter = MetadataFilter::In("status".into(), vec![json!("active"), json!("pending")]);
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_in_does_not_match() {
        let meta = make_metadata(&[("status", json!("archived"))]);
        let filter = MetadataFilter::In("status".into(), vec![json!("active"), json!("pending")]);
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn filter_gt_matches() {
        let meta = make_metadata(&[("score", json!(85))]);
        let filter = MetadataFilter::Gt("score".into(), json!(80));
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_lt_matches() {
        let meta = make_metadata(&[("score", json!(50))]);
        let filter = MetadataFilter::Lt("score".into(), json!(80));
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_and_all_match() {
        let meta = make_metadata(&[("color", json!("red")), ("score", json!(90))]);
        let filter = MetadataFilter::And(vec![
            MetadataFilter::Eq("color".into(), json!("red")),
            MetadataFilter::Gt("score".into(), json!(80)),
        ]);
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_and_partial_match_fails() {
        let meta = make_metadata(&[("color", json!("blue")), ("score", json!(90))]);
        let filter = MetadataFilter::And(vec![
            MetadataFilter::Eq("color".into(), json!("red")),
            MetadataFilter::Gt("score".into(), json!(80)),
        ]);
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn filter_or_one_matches() {
        let meta = make_metadata(&[("color", json!("blue"))]);
        let filter = MetadataFilter::Or(vec![
            MetadataFilter::Eq("color".into(), json!("red")),
            MetadataFilter::Eq("color".into(), json!("blue")),
        ]);
        assert!(filter.matches(&meta));
    }

    #[test]
    fn filter_or_none_match() {
        let meta = make_metadata(&[("color", json!("green"))]);
        let filter = MetadataFilter::Or(vec![
            MetadataFilter::Eq("color".into(), json!("red")),
            MetadataFilter::Eq("color".into(), json!("blue")),
        ]);
        assert!(!filter.matches(&meta));
    }
}

//! Message filtering by type, name, or ID.

use crate::messages::Message;

/// Builder for filtering messages by type, name, or ID.
///
/// All conditions within a single field (e.g. multiple include types) are
/// combined with OR. Conditions across different fields are combined with AND.
///
/// # Examples
///
/// ```
/// use synwire_core::messages::{Message, MessageFilter};
///
/// let messages = vec![
///     Message::human("Hello"),
///     Message::ai("Hi there"),
///     Message::system("Be helpful"),
/// ];
///
/// let filter = MessageFilter::new()
///     .include_types(vec!["human".to_string()]);
/// let filtered = filter.filter(&messages);
/// assert_eq!(filtered.len(), 1);
/// assert_eq!(filtered[0].message_type(), "human");
/// ```
#[derive(Debug, Default)]
pub struct MessageFilter {
    include_types: Option<Vec<String>>,
    exclude_types: Option<Vec<String>>,
    include_names: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    include_ids: Option<Vec<String>>,
    exclude_ids: Option<Vec<String>>,
}

impl MessageFilter {
    /// Create a new filter with no constraints (passes all messages).
    pub fn new() -> Self {
        Self::default()
    }

    /// Only include messages whose type is in the given list.
    #[must_use]
    pub fn include_types(mut self, types: Vec<String>) -> Self {
        self.include_types = Some(types);
        self
    }

    /// Exclude messages whose type is in the given list.
    #[must_use]
    pub fn exclude_types(mut self, types: Vec<String>) -> Self {
        self.exclude_types = Some(types);
        self
    }

    /// Only include messages whose name is in the given list.
    #[must_use]
    pub fn include_names(mut self, names: Vec<String>) -> Self {
        self.include_names = Some(names);
        self
    }

    /// Exclude messages whose name is in the given list.
    #[must_use]
    pub fn exclude_names(mut self, names: Vec<String>) -> Self {
        self.exclude_names = Some(names);
        self
    }

    /// Only include messages whose ID is in the given list.
    #[must_use]
    pub fn include_ids(mut self, ids: Vec<String>) -> Self {
        self.include_ids = Some(ids);
        self
    }

    /// Exclude messages whose ID is in the given list.
    #[must_use]
    pub fn exclude_ids(mut self, ids: Vec<String>) -> Self {
        self.exclude_ids = Some(ids);
        self
    }

    /// Apply filter to a list of messages, returning those that match.
    pub fn filter(&self, messages: &[Message]) -> Vec<Message> {
        messages
            .iter()
            .filter(|msg| self.matches(msg))
            .cloned()
            .collect()
    }

    /// Check whether a single message matches all filter criteria.
    fn matches(&self, msg: &Message) -> bool {
        let msg_type = msg.message_type();
        let msg_name = msg.name();
        let msg_id = msg.id();

        if let Some(ref include) = self.include_types
            && !include.iter().any(|t| t == msg_type)
        {
            return false;
        }
        if let Some(ref exclude) = self.exclude_types
            && exclude.iter().any(|t| t == msg_type)
        {
            return false;
        }

        if let Some(ref include) = self.include_names {
            match msg_name {
                Some(name) => {
                    if !include.iter().any(|n| n == name) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if let Some(ref exclude) = self.exclude_names
            && let Some(name) = msg_name
            && exclude.iter().any(|n| n == name)
        {
            return false;
        }

        if let Some(ref include) = self.include_ids {
            match msg_id {
                Some(id) => {
                    if !include.iter().any(|i| i == id) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if let Some(ref exclude) = self.exclude_ids
            && let Some(id) = msg_id
            && exclude.iter().any(|i| i == id)
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_by_type() {
        let messages = vec![
            Message::human("Hello"),
            Message::ai("Hi"),
            Message::system("Be helpful"),
            Message::human("How are you?"),
        ];

        let filter = MessageFilter::new().include_types(vec!["human".to_string()]);
        let filtered = filter.filter(&messages);
        assert_eq!(filtered.len(), 2);
        for msg in &filtered {
            assert_eq!(msg.message_type(), "human");
        }
    }

    #[test]
    fn test_filter_exclude_type() {
        let messages = vec![
            Message::human("Hello"),
            Message::ai("Hi"),
            Message::system("Be helpful"),
        ];

        let filter = MessageFilter::new().exclude_types(vec!["system".to_string()]);
        let filtered = filter.filter(&messages);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|m| m.message_type() != "system"));
    }

    #[test]
    fn test_filter_no_constraints_passes_all() {
        let messages = vec![Message::human("Hello"), Message::ai("Hi")];

        let filter = MessageFilter::new();
        let filtered = filter.filter(&messages);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_multiple_types() {
        let messages = vec![
            Message::human("Hello"),
            Message::ai("Hi"),
            Message::system("Be helpful"),
        ];

        let filter =
            MessageFilter::new().include_types(vec!["human".to_string(), "ai".to_string()]);
        let filtered = filter.filter(&messages);
        assert_eq!(filtered.len(), 2);
    }
}

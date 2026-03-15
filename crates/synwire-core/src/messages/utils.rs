//! Message utility functions for trimming and merging.

use crate::messages::{Message, MessageContent};

/// Strategy for trimming messages.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrimStrategy {
    /// Remove from the beginning (keep most recent).
    First,
    /// Remove from the end (keep oldest).
    Last,
}

/// Estimate token count for a message using character-based approximation.
///
/// Uses a simple heuristic: 1 token is approximately 4 characters.
fn estimate_tokens(msg: &Message) -> usize {
    let text_len = msg.content().as_text().len();
    text_len.div_ceil(4)
}

/// Trim messages to fit within a token budget.
///
/// Uses a simple approximation: 1 token is approximately 4 characters.
/// When the total estimated tokens exceed `max_tokens`, messages are removed
/// according to the chosen strategy.
///
/// # Examples
///
/// ```
/// use synwire_core::messages::{Message, trim_messages, TrimStrategy};
///
/// let messages = vec![
///     Message::system("You are helpful"),
///     Message::human("Hello"),
///     Message::ai("Hi there! How can I help you today?"),
/// ];
///
/// // Keep only messages that fit in 10 tokens
/// let trimmed = trim_messages(&messages, 10, &TrimStrategy::First);
/// assert!(trimmed.len() <= messages.len());
/// ```
pub fn trim_messages(
    messages: &[Message],
    max_tokens: usize,
    strategy: &TrimStrategy,
) -> Vec<Message> {
    let total: usize = messages.iter().map(estimate_tokens).sum();

    if total <= max_tokens {
        return messages.to_vec();
    }

    match strategy {
        TrimStrategy::First => {
            // Remove from the beginning, keep most recent
            let mut result = Vec::new();
            let mut budget = max_tokens;

            for msg in messages.iter().rev() {
                let tokens = estimate_tokens(msg);
                if tokens <= budget {
                    result.push(msg.clone());
                    budget -= tokens;
                } else {
                    break;
                }
            }

            result.reverse();
            result
        }
        TrimStrategy::Last => {
            // Remove from the end, keep oldest
            let mut result = Vec::new();
            let mut budget = max_tokens;

            for msg in messages {
                let tokens = estimate_tokens(msg);
                if tokens <= budget {
                    result.push(msg.clone());
                    budget -= tokens;
                } else {
                    break;
                }
            }

            result
        }
    }
}

/// Merge consecutive messages of the same type into single messages.
///
/// When multiple messages of the same type appear consecutively, their text
/// content is concatenated with newlines. Non-consecutive messages of the
/// same type are not merged.
///
/// # Examples
///
/// ```
/// use synwire_core::messages::{Message, merge_message_runs};
///
/// let messages = vec![
///     Message::human("Hello"),
///     Message::human("How are you?"),
///     Message::ai("I'm fine"),
/// ];
///
/// let merged = merge_message_runs(&messages);
/// assert_eq!(merged.len(), 2);
/// assert_eq!(merged[0].content().as_text(), "Hello\nHow are you?");
/// ```
pub fn merge_message_runs(messages: &[Message]) -> Vec<Message> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<Message> = Vec::new();

    for msg in messages {
        let should_merge = result
            .last()
            .is_some_and(|last| last.message_type() == msg.message_type());

        if should_merge {
            // Merge content with the last message
            if let Some(last) = result.last_mut() {
                let combined = format!("{}\n{}", last.content().as_text(), msg.content().as_text());
                let new_content = MessageContent::Text(combined);
                replace_content(last, new_content);
            }
        } else {
            result.push(msg.clone());
        }
    }

    result
}

/// Replace the content of a message in-place.
fn replace_content(msg: &mut Message, new_content: MessageContent) {
    match msg {
        Message::Human { content, .. }
        | Message::AI { content, .. }
        | Message::System { content, .. }
        | Message::Tool { content, .. }
        | Message::Chat { content, .. } => {
            *content = new_content;
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_messages_first_strategy() {
        // Create messages where total tokens exceed budget
        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::human("Hello there"),
            Message::ai("Hi"),
        ];

        // Use a small budget that can only fit the last message
        let trimmed = trim_messages(&messages, 2, &TrimStrategy::First);
        // Should keep only most recent messages that fit
        assert!(!trimmed.is_empty());
        assert!(trimmed.len() < messages.len());
        // Last message should be the AI message
        assert_eq!(trimmed.last().map(Message::message_type), Some("ai"));
    }

    #[test]
    fn test_trim_messages_last_strategy() {
        let messages = vec![
            Message::human("Hi"),
            Message::ai("Hello! How can I help you today with your questions?"),
            Message::human("Tell me about Rust"),
        ];

        // Small budget keeps only first messages
        let trimmed = trim_messages(&messages, 3, &TrimStrategy::Last);
        assert!(!trimmed.is_empty());
        assert_eq!(trimmed[0].message_type(), "human");
    }

    #[test]
    fn test_trim_messages_within_budget() {
        let messages = vec![Message::human("Hi"), Message::ai("Hello")];

        // Large budget keeps everything
        let trimmed = trim_messages(&messages, 1000, &TrimStrategy::First);
        assert_eq!(trimmed.len(), 2);
    }

    #[test]
    fn test_trim_messages_empty() {
        let trimmed = trim_messages(&[], 100, &TrimStrategy::First);
        assert!(trimmed.is_empty());
    }

    #[test]
    fn test_merge_message_runs() {
        let messages = vec![
            Message::human("Hello"),
            Message::human("How are you?"),
            Message::ai("I'm fine"),
            Message::ai("Thanks for asking"),
            Message::human("Great"),
        ];

        let merged = merge_message_runs(&messages);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].content().as_text(), "Hello\nHow are you?");
        assert_eq!(merged[1].content().as_text(), "I'm fine\nThanks for asking");
        assert_eq!(merged[2].content().as_text(), "Great");
    }

    #[test]
    fn test_merge_message_runs_no_consecutive() {
        let messages = vec![
            Message::human("Hello"),
            Message::ai("Hi"),
            Message::human("How are you?"),
        ];

        let merged = merge_message_runs(&messages);
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn test_merge_message_runs_empty() {
        let merged = merge_message_runs(&[]);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_merge_message_runs_single() {
        let messages = vec![Message::human("Hello")];
        let merged = merge_message_runs(&messages);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].content().as_text(), "Hello");
    }
}

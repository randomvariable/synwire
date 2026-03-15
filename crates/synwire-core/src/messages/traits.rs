//! Trait for types convertible to a [`Message`].

use crate::messages::Message;

/// Trait for types that can be converted to a [`Message`].
///
/// This provides a uniform interface for accepting various types as messages,
/// such as raw strings, tuples of (role, content), or `Message` values directly.
///
/// # Examples
///
/// ```
/// use synwire_core::messages::{Message, MessageLike};
///
/// let msg: Message = "Hello".to_message();
/// assert_eq!(msg.message_type(), "human");
///
/// let msg: Message = ("ai", "Hi there").to_message();
/// assert_eq!(msg.message_type(), "ai");
/// ```
pub trait MessageLike {
    /// Convert to a [`Message`].
    fn to_message(&self) -> Message;
}

impl MessageLike for Message {
    fn to_message(&self) -> Message {
        self.clone()
    }
}

impl MessageLike for &str {
    fn to_message(&self) -> Message {
        Message::human(*self)
    }
}

impl MessageLike for String {
    fn to_message(&self) -> Message {
        Message::human(self)
    }
}

impl MessageLike for (&str, &str) {
    /// First element is role (`"human"`, `"user"`, `"ai"`, `"assistant"`, `"system"`),
    /// second is content. Unrecognised roles default to human.
    fn to_message(&self) -> Message {
        match self.0 {
            "ai" | "assistant" => Message::ai(self.1),
            "system" => Message::system(self.1),
            // "human", "user", or any unrecognised role defaults to human.
            _ => Message::human(self.1),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_message_like_from_str() {
        let msg = "Hello".to_message();
        assert_eq!(msg.message_type(), "human");
        assert_eq!(msg.content().as_text(), "Hello");
    }

    #[test]
    fn test_message_like_from_string() {
        let s = String::from("Hello");
        let msg = s.to_message();
        assert_eq!(msg.message_type(), "human");
    }

    #[test]
    fn test_message_like_from_message() {
        let original = Message::ai("Hi");
        let msg = original.to_message();
        assert_eq!(msg.message_type(), "ai");
    }

    #[test]
    fn test_message_like_from_tuple_human() {
        let msg = ("human", "Hello").to_message();
        assert_eq!(msg.message_type(), "human");
    }

    #[test]
    fn test_message_like_from_tuple_user() {
        let msg = ("user", "Hello").to_message();
        assert_eq!(msg.message_type(), "human");
    }

    #[test]
    fn test_message_like_from_tuple_ai() {
        let msg = ("ai", "Hi").to_message();
        assert_eq!(msg.message_type(), "ai");
    }

    #[test]
    fn test_message_like_from_tuple_assistant() {
        let msg = ("assistant", "Hi").to_message();
        assert_eq!(msg.message_type(), "ai");
    }

    #[test]
    fn test_message_like_from_tuple_system() {
        let msg = ("system", "Be helpful").to_message();
        assert_eq!(msg.message_type(), "system");
    }

    #[test]
    fn test_message_like_from_tuple_unknown_defaults_to_human() {
        let msg = ("unknown_role", "Hello").to_message();
        assert_eq!(msg.message_type(), "human");
    }
}

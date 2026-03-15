//! Prompt value type.

use crate::messages::Message;
use serde::{Deserialize, Serialize};

/// A formatted prompt ready for model consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PromptValue {
    /// Plain text prompt.
    String(String),
    /// Message-based prompt.
    Messages(Vec<Message>),
}

impl PromptValue {
    /// Converts the prompt value to a string representation.
    pub fn to_text(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Messages(messages) => messages
                .iter()
                .map(|m| format!("{}: {}", m.message_type(), m.content().as_text()))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// Converts the prompt value to a list of messages.
    pub fn to_messages(&self) -> Vec<Message> {
        match self {
            Self::String(s) => vec![Message::human(s.clone())],
            Self::Messages(messages) => messages.clone(),
        }
    }
}

impl std::fmt::Display for PromptValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

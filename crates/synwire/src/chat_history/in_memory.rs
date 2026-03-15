//! In-memory chat message history implementation.

use std::sync::Mutex;

use synwire_core::BoxFuture;
use synwire_core::error::SynwireError;
use synwire_core::messages::Message;

use super::traits::ChatMessageHistory;

/// In-memory chat message history backed by a `Vec<Message>`.
///
/// Suitable for testing and single-session use cases.
///
/// # Examples
///
/// ```
/// use synwire::chat_history::{ChatMessageHistory, InMemoryChatMessageHistory};
/// use synwire_core::messages::Message;
///
/// # tokio_test::block_on(async {
/// let history = InMemoryChatMessageHistory::new();
/// history.add_message(Message::human("Hello")).await.unwrap();
/// let msgs = history.get_messages().await.unwrap();
/// assert_eq!(msgs.len(), 1);
/// # });
/// ```
pub struct InMemoryChatMessageHistory {
    messages: Mutex<Vec<Message>>,
}

impl InMemoryChatMessageHistory {
    /// Creates a new empty in-memory chat history.
    pub const fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryChatMessageHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to convert a mutex poison error to a `SynwireError`.
fn lock_err(e: impl std::fmt::Display) -> SynwireError {
    SynwireError::Other(Box::new(std::io::Error::other(e.to_string())))
}

impl ChatMessageHistory for InMemoryChatMessageHistory {
    fn get_messages(&self) -> BoxFuture<'_, Result<Vec<Message>, SynwireError>> {
        Box::pin(async move {
            let guard = self.messages.lock().map_err(lock_err)?;
            Ok(guard.clone())
        })
    }

    fn add_message(&self, message: Message) -> BoxFuture<'_, Result<(), SynwireError>> {
        Box::pin(async move {
            self.messages.lock().map_err(lock_err)?.push(message);
            Ok(())
        })
    }

    fn clear(&self) -> BoxFuture<'_, Result<(), SynwireError>> {
        Box::pin(async move {
            self.messages.lock().map_err(lock_err)?.clear();
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_history_is_empty() {
        let history = InMemoryChatMessageHistory::new();
        let msgs = history.get_messages().await.unwrap();
        assert!(msgs.is_empty());
    }

    #[tokio::test]
    async fn add_and_retrieve_messages() {
        let history = InMemoryChatMessageHistory::new();
        history.add_message(Message::human("Hi")).await.unwrap();
        history.add_message(Message::ai("Hello")).await.unwrap();

        let msgs = history.get_messages().await.unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].message_type(), "human");
        assert_eq!(msgs[1].message_type(), "ai");
    }

    #[tokio::test]
    async fn clear_removes_all_messages() {
        let history = InMemoryChatMessageHistory::new();
        history.add_message(Message::human("Hi")).await.unwrap();
        history.clear().await.unwrap();

        let msgs = history.get_messages().await.unwrap();
        assert!(msgs.is_empty());
    }
}

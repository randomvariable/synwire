//! Chat message history trait definition.

use synwire_core::BoxFuture;
use synwire_core::error::SynwireError;
use synwire_core::messages::Message;

/// Trait for managing a session's chat message history.
///
/// Implementations store and retrieve messages for a given session,
/// enabling multi-turn conversations.
pub trait ChatMessageHistory: Send + Sync {
    /// Returns all messages for this history.
    fn get_messages(&self) -> BoxFuture<'_, Result<Vec<Message>, SynwireError>>;

    /// Appends a message to the history.
    fn add_message(&self, message: Message) -> BoxFuture<'_, Result<(), SynwireError>>;

    /// Clears all messages from the history.
    fn clear(&self) -> BoxFuture<'_, Result<(), SynwireError>>;
}

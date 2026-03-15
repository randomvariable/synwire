//! Message handling utilities for graph state.
//!
//! Provides reducer functions for accumulating message lists in graph state,
//! and [`MessagesState`] -- a built-in typed state for chat-based agents.

use serde::{Deserialize, Serialize};
use synwire_core::messages::Message;

/// Built-in state type for chat-based agents.
///
/// Contains a `messages` field with [`Topic`](crate::channels::Topic) channel
/// semantics (append, not replace).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesState {
    /// Conversation history.
    pub messages: Vec<Message>,
}

impl crate::graph::state::State for MessagesState {
    fn channels() -> Vec<(String, Box<dyn crate::channels::BaseChannel>)> {
        vec![(
            "messages".to_owned(),
            Box::new(crate::channels::Topic::new("messages"))
                as Box<dyn crate::channels::BaseChannel>,
        )]
    }

    fn from_channels(
        channels: &std::collections::HashMap<String, Box<dyn crate::channels::BaseChannel>>,
    ) -> Result<Self, crate::error::GraphError> {
        let messages = channels
            .get("messages")
            .map(|c| {
                let v = c.checkpoint();
                serde_json::from_value(v)
            })
            .transpose()
            .map_err(|e| crate::error::GraphError::DeserializationError {
                field: "messages".into(),
                message: e.to_string(),
            })?
            .unwrap_or_default();
        Ok(Self { messages })
    }
}

/// Creates a reducer function that appends messages to a list.
///
/// The reducer expects both the current value and the incoming value to be
/// JSON arrays. It concatenates them in order.
pub fn add_messages(
    current: &serde_json::Value,
    incoming: &serde_json::Value,
) -> serde_json::Value {
    let mut result = match current {
        serde_json::Value::Array(arr) => arr.clone(),
        _ => vec![current.clone()],
    };

    match incoming {
        serde_json::Value::Array(arr) => result.extend(arr.iter().cloned()),
        other => result.push(other.clone()),
    }

    serde_json::Value::Array(result)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn appends_arrays() {
        let a = serde_json::json!(["hello"]);
        let b = serde_json::json!(["world"]);
        let result = add_messages(&a, &b);
        assert_eq!(result, serde_json::json!(["hello", "world"]));
    }

    #[test]
    fn appends_single_to_array() {
        let a = serde_json::json!(["hello"]);
        let b = serde_json::json!("world");
        let result = add_messages(&a, &b);
        assert_eq!(result, serde_json::json!(["hello", "world"]));
    }

    /// T024: `MessagesState` implements State with Topic channel on messages.
    #[test]
    fn t024_messages_state_has_topic_channel() {
        use crate::graph::state::State;

        let channels = MessagesState::channels();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].0, "messages");
    }

    /// Verify `channels()` returns one entry keyed "messages" and the channel
    /// accumulates values (Topic semantics).
    #[test]
    fn messages_state_topic_accumulates() {
        use crate::graph::state::State;

        let mut channels_vec = MessagesState::channels();
        assert_eq!(channels_vec.len(), 1);

        let (ref key, ref mut channel) = channels_vec[0];
        assert_eq!(key, "messages");

        // Write two messages in sequence; Topic should accumulate both.
        let msg1 = serde_json::json!({"type": "human", "content": "hello"});
        let msg2 = serde_json::json!({"type": "human", "content": "world"});
        channel.update(vec![msg1]).unwrap();
        channel.update(vec![msg2]).unwrap();

        let checkpoint = channel.checkpoint();
        let arr = checkpoint.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    /// Round-trip: serialise to value and back.
    #[test]
    fn messages_state_round_trip() {
        use crate::graph::state::State;

        let state = MessagesState { messages: vec![] };
        let value = state.to_value().unwrap();
        let restored = MessagesState::from_value(value).unwrap();
        assert!(restored.messages.is_empty());
    }
}

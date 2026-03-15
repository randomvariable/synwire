//! Stream events and content categories.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// An event emitted during runnable execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum StreamEvent {
    /// Standard lifecycle event.
    #[serde(rename = "standard")]
    Standard {
        /// Event name (e.g. `on_chain_start`).
        event: String,
        /// Runnable name.
        name: String,
        /// Run identifier.
        run_id: String,
        /// Parent run identifiers.
        #[serde(default)]
        parent_ids: Vec<String>,
        /// Tags.
        #[serde(default)]
        tags: Vec<String>,
        /// Metadata.
        #[serde(default)]
        metadata: HashMap<String, Value>,
        /// Event data.
        data: EventData,
    },
    /// Custom user-dispatched event.
    #[serde(rename = "custom")]
    Custom {
        /// Event name.
        name: String,
        /// Run identifier.
        run_id: String,
        /// Parent run identifiers.
        #[serde(default)]
        parent_ids: Vec<String>,
        /// Tags.
        #[serde(default)]
        tags: Vec<String>,
        /// Metadata.
        #[serde(default)]
        metadata: HashMap<String, Value>,
        /// Custom event data.
        data: Value,
    },
}

/// Data payload for standard stream events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventData {
    /// Input data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    /// Output data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Chunk data (for streaming events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk: Option<Value>,
    /// Error description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Content category distinguishing primary response content from
    /// secondary metadata such as tool calls and usage metrics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<ContentCategory>,
}

/// Dispatch a custom event.
///
/// Creates a [`StreamEvent::Custom`] with the given name and data.
/// The `run_id` is set to an empty string and can be overridden by the
/// event bus or callback infrastructure.
pub fn dispatch_custom_event(name: impl Into<String>, data: serde_json::Value) -> StreamEvent {
    StreamEvent::Custom {
        name: name.into(),
        run_id: String::new(),
        parent_ids: Vec::new(),
        tags: Vec::new(),
        metadata: HashMap::new(),
        data,
    }
}

/// Distinguishes primary from secondary content in streaming responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ContentCategory {
    /// Actual response content: text, structured data.
    Primary,
    /// Intermediate: tool calls, reasoning, usage metrics.
    Secondary,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_custom_event() {
        let data = serde_json::json!({"key": "value"});
        let event = dispatch_custom_event("my_event", data.clone());
        match event {
            StreamEvent::Custom { name, data: d, .. } => {
                assert_eq!(name, "my_event");
                assert_eq!(d, data);
            }
            _ => panic!("expected Custom variant"),
        }
    }

    #[test]
    fn test_dispatch_custom_event_empty_defaults() {
        let event = dispatch_custom_event("test", serde_json::Value::Null);
        match event {
            StreamEvent::Custom {
                run_id,
                parent_ids,
                tags,
                metadata,
                ..
            } => {
                assert!(run_id.is_empty());
                assert!(parent_ids.is_empty());
                assert!(tags.is_empty());
                assert!(metadata.is_empty());
            }
            _ => panic!("expected Custom variant"),
        }
    }
}

//! Message types for the Synwire framework.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A single unit of conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum Message {
    /// A human message.
    #[serde(rename = "human")]
    Human {
        /// Unique message identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Optional sender name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Message content.
        content: MessageContent,
        /// Additional provider-specific kwargs.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
    },
    /// An AI message.
    #[serde(rename = "ai")]
    AI {
        /// Unique message identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Optional sender name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Message content.
        content: MessageContent,
        /// Tool calls made by the model.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
        /// Invalid tool calls attempted by the model.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        invalid_tool_calls: Vec<InvalidToolCall>,
        /// Token usage statistics.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<UsageMetadata>,
        /// Provider-specific response metadata.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response_metadata: Option<HashMap<String, Value>>,
        /// Additional provider-specific kwargs.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
    },
    /// A system message.
    #[serde(rename = "system")]
    System {
        /// Unique message identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Optional sender name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Message content.
        content: MessageContent,
        /// Additional provider-specific kwargs.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
    },
    /// A tool response message.
    #[serde(rename = "tool")]
    Tool {
        /// Unique message identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Optional sender name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Message content.
        content: MessageContent,
        /// The ID of the tool call this responds to.
        tool_call_id: String,
        /// Tool execution status.
        #[serde(default)]
        status: ToolStatus,
        /// Rich tool output not sent to the model.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        artifact: Option<Value>,
        /// Additional provider-specific kwargs.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
    },
    /// A generic chat message with custom role.
    #[serde(rename = "chat")]
    Chat {
        /// Unique message identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Optional sender name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Arbitrary role string.
        role: String,
        /// Message content.
        content: MessageContent,
        /// Additional provider-specific kwargs.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        additional_kwargs: HashMap<String, Value>,
    },
}

impl Message {
    /// Creates a new human message from text.
    pub fn human(text: impl Into<String>) -> Self {
        Self::Human {
            id: None,
            name: None,
            content: MessageContent::Text(text.into()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Creates a new AI message from text.
    pub fn ai(text: impl Into<String>) -> Self {
        Self::AI {
            id: None,
            name: None,
            content: MessageContent::Text(text.into()),
            tool_calls: Vec::new(),
            invalid_tool_calls: Vec::new(),
            usage: None,
            response_metadata: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Creates a new system message from text.
    pub fn system(text: impl Into<String>) -> Self {
        Self::System {
            id: None,
            name: None,
            content: MessageContent::Text(text.into()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Creates a new tool message.
    pub fn tool(text: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self::Tool {
            id: None,
            name: None,
            content: MessageContent::Text(text.into()),
            tool_call_id: tool_call_id.into(),
            status: ToolStatus::Success,
            artifact: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Returns the message ID, if set.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Human { id, .. }
            | Self::AI { id, .. }
            | Self::System { id, .. }
            | Self::Tool { id, .. }
            | Self::Chat { id, .. } => id.as_deref(),
        }
    }

    /// Returns the sender name, if set.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Human { name, .. }
            | Self::AI { name, .. }
            | Self::System { name, .. }
            | Self::Tool { name, .. }
            | Self::Chat { name, .. } => name.as_deref(),
        }
    }

    /// Returns the content of the message.
    pub const fn content(&self) -> &MessageContent {
        match self {
            Self::Human { content, .. }
            | Self::AI { content, .. }
            | Self::System { content, .. }
            | Self::Tool { content, .. }
            | Self::Chat { content, .. } => content,
        }
    }

    /// Returns the message type as a string.
    pub const fn message_type(&self) -> &str {
        match self {
            Self::Human { .. } => "human",
            Self::AI { .. } => "ai",
            Self::System { .. } => "system",
            Self::Tool { .. } => "tool",
            Self::Chat { .. } => "chat",
        }
    }
}

/// Message content — either plain text or structured blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum MessageContent {
    /// Plain text content.
    Text(String),
    /// Structured content blocks.
    Blocks(Vec<ContentBlock>),
}

impl MessageContent {
    /// Returns the text representation of the content.
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

/// A single block of structured content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum ContentBlock {
    /// Text content block.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// Image content block.
    #[serde(rename = "image")]
    Image {
        /// Image URL.
        url: String,
        /// Detail level hint.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Audio content block.
    #[serde(rename = "audio")]
    Audio {
        /// Audio URL.
        url: String,
        /// MIME type.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// Video content block.
    #[serde(rename = "video")]
    Video {
        /// Video URL.
        url: String,
        /// MIME type.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// File content block.
    #[serde(rename = "file")]
    File {
        /// File URL.
        url: String,
        /// MIME type.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// Chain-of-thought reasoning from models.
    #[serde(rename = "reasoning")]
    Reasoning {
        /// The reasoning text.
        text: String,
    },
    /// Model thinking/scratchpad content.
    #[serde(rename = "thinking")]
    Thinking {
        /// The thinking text.
        text: String,
    },
}

/// A structured request from a model to invoke a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique tool call identifier.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Tool arguments as key-value pairs.
    pub arguments: HashMap<String, Value>,
}

/// A tool call that failed to produce valid arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidToolCall {
    /// Tool name, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Raw unparsed arguments string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// Tool call identifier, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Error description.
    pub error: String,
}

/// Tool execution status.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ToolStatus {
    /// Tool executed successfully.
    #[default]
    Success,
    /// Tool execution failed.
    Error,
}

/// Token usage statistics from a model invocation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageMetadata {
    /// Number of input tokens.
    pub input_tokens: u64,
    /// Number of output tokens.
    pub output_tokens: u64,
    /// Total number of tokens.
    pub total_tokens: u64,
    /// Detailed input token breakdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_token_details: Option<InputTokenDetails>,
    /// Detailed output token breakdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_token_details: Option<OutputTokenDetails>,
}

/// Detailed breakdown of input token usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputTokenDetails {
    /// Audio input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<u64>,
    /// Cache creation tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<u64>,
    /// Cache read tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u64>,
}

/// Detailed breakdown of output token usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputTokenDetails {
    /// Audio output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<u64>,
    /// Reasoning output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_human_message_construction() {
        let msg = Message::human("Hello");
        assert_eq!(msg.message_type(), "human");
        assert_eq!(msg.content().as_text(), "Hello");
    }

    #[test]
    fn test_ai_message_construction() {
        let msg = Message::ai("World");
        assert_eq!(msg.message_type(), "ai");
        assert_eq!(msg.content().as_text(), "World");
    }

    #[test]
    fn test_system_message_construction() {
        let msg = Message::system("Be helpful");
        assert_eq!(msg.message_type(), "system");
        assert_eq!(msg.content().as_text(), "Be helpful");
    }

    #[test]
    fn test_tool_message_construction() {
        let msg = Message::tool("result", "call_123");
        assert_eq!(msg.message_type(), "tool");
        assert_eq!(msg.content().as_text(), "result");
    }

    #[test]
    fn test_message_serde_roundtrip_human() {
        let msg = Message::human("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message_type(), "human");
        assert_eq!(deserialized.content().as_text(), "Hello");
    }

    #[test]
    fn test_message_serde_roundtrip_ai_with_tool_calls() {
        let msg = Message::AI {
            id: Some("msg_1".into()),
            name: None,
            content: MessageContent::Text("Let me call a tool".into()),
            tool_calls: vec![ToolCall {
                id: "tc_1".into(),
                name: "search".into(),
                arguments: {
                    let mut m = std::collections::HashMap::new();
                    let _ = m.insert("query".into(), serde_json::Value::String("rust".into()));
                    m
                },
            }],
            invalid_tool_calls: Vec::new(),
            usage: Some(UsageMetadata {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
                ..Default::default()
            }),
            response_metadata: None,
            additional_kwargs: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message_type(), "ai");
        #[allow(clippy::panic)]
        if let Message::AI {
            tool_calls, usage, ..
        } = &deserialized
        {
            assert_eq!(tool_calls.len(), 1);
            assert_eq!(tool_calls[0].name, "search");
            assert_eq!(usage.as_ref().map(|u| u.input_tokens), Some(10));
        } else {
            panic!("Expected AI message");
        }
    }

    #[test]
    fn test_message_content_blocks() {
        let content = MessageContent::Blocks(vec![
            ContentBlock::Text {
                text: "Hello ".into(),
            },
            ContentBlock::Text {
                text: "World".into(),
            },
        ]);
        assert_eq!(content.as_text(), "Hello World");
    }
}

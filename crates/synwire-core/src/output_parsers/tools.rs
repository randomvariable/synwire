//! Tools output parser that extracts tool calls from messages.

use crate::error::{ParseError, SynwireError};
use crate::messages::{Message, ToolCall};
use crate::output_parsers::OutputParser;

/// Parser that extracts tool calls from an AI message.
///
/// Can operate in two modes:
/// - **Message-based**: Extracts `ToolCall` values directly from an AI message variant.
/// - **Text-based**: Parses a JSON array of tool calls from raw text (via the `OutputParser` trait).
///
/// # Examples
///
/// ```
/// use synwire_core::output_parsers::ToolsOutputParser;
/// use synwire_core::messages::Message;
///
/// let parser = ToolsOutputParser;
/// let msg = Message::ai("No tools needed");
/// let calls = parser.parse_message(&msg).unwrap();
/// assert!(calls.is_empty());
/// ```
pub struct ToolsOutputParser;

impl ToolsOutputParser {
    /// Extract tool calls from a message.
    ///
    /// Returns the tool calls if the message is an AI message, or an empty
    /// vector for any other message type.
    ///
    /// # Errors
    ///
    /// This method currently does not produce errors, but returns `Result`
    /// for forward compatibility.
    pub fn parse_message(&self, message: &Message) -> Result<Vec<ToolCall>, SynwireError> {
        match message {
            Message::AI { tool_calls, .. } => Ok(tool_calls.clone()),
            _ => Ok(Vec::new()),
        }
    }
}

impl OutputParser for ToolsOutputParser {
    type Output = Vec<ToolCall>;

    fn parse(&self, text: &str) -> Result<Vec<ToolCall>, SynwireError> {
        serde_json::from_str(text).map_err(|e| {
            SynwireError::from(ParseError::ParseFailed {
                message: format!("Failed to parse tool calls: {e}"),
            })
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_tools_parser_extracts_calls() {
        let parser = ToolsOutputParser;
        let msg = Message::AI {
            id: None,
            name: None,
            content: crate::messages::MessageContent::Text("Calling tool".into()),
            tool_calls: vec![ToolCall {
                id: "tc_1".into(),
                name: "search".into(),
                arguments: {
                    let mut m = HashMap::new();
                    let _ = m.insert("query".into(), serde_json::Value::String("rust".into()));
                    m
                },
            }],
            invalid_tool_calls: Vec::new(),
            usage: None,
            response_metadata: None,
            additional_kwargs: HashMap::new(),
        };
        let calls = parser.parse_message(&msg).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "search");
    }

    #[test]
    fn test_tools_parser_non_ai_message() {
        let parser = ToolsOutputParser;
        let msg = Message::human("Hello");
        let calls = parser.parse_message(&msg).unwrap();
        assert!(calls.is_empty());
    }

    #[test]
    fn test_tools_parser_from_text() {
        let parser = ToolsOutputParser;
        let json = r#"[{"id": "tc_1", "name": "search", "arguments": {"query": "test"}}]"#;
        let calls = parser.parse(json).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "tc_1");
    }

    #[test]
    fn test_tools_parser_invalid_text() {
        let parser = ToolsOutputParser;
        let result = parser.parse("not json");
        assert!(result.is_err());
    }
}

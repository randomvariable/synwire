//! Language model result and streaming types.

use crate::messages::{Message, UsageMetadata};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// The output of a chat model invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    /// The AI response message.
    pub message: Message,
    /// Additional generation info from the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,
    /// Estimated cost of this invocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostEstimate>,
}

/// Estimated monetary cost of a model invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Input cost in the specified currency.
    pub input_cost: f64,
    /// Output cost in the specified currency.
    pub output_cost: f64,
    /// Total cost in the specified currency.
    pub total_cost: f64,
    /// ISO 4217 currency code.
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

/// The output of a batch LLM invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResult {
    /// Generations for each input.
    pub generations: Vec<Vec<Generation>>,
    /// Additional LLM output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_output: Option<HashMap<String, Value>>,
}

/// A single generation from a text LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    /// Generated text.
    pub text: String,
    /// Additional generation info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,
}

/// A single chunk of streaming output from a chat model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatChunk {
    /// Incremental text content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_content: Option<String>,
    /// Incremental tool call data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delta_tool_calls: Vec<ToolCallChunk>,
    /// Finish reason, if generation is complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Usage statistics, if provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageMetadata>,
}

impl ChatChunk {
    /// Merges another chunk into this one.
    ///
    /// Concatenates content, merges tool calls by index,
    /// and takes the last non-None finish reason and usage.
    pub fn merge(&mut self, other: &Self) {
        // Merge content
        if let Some(ref other_content) = other.delta_content {
            match self.delta_content {
                Some(ref mut content) => content.push_str(other_content),
                None => self.delta_content = Some(other_content.clone()),
            }
        }

        // Merge tool call chunks by index
        for other_tc in &other.delta_tool_calls {
            if let Some(existing) = self
                .delta_tool_calls
                .iter_mut()
                .find(|tc| tc.index == other_tc.index)
            {
                existing.merge(other_tc);
            } else {
                self.delta_tool_calls.push(other_tc.clone());
            }
        }

        // Take last non-None values
        if other.finish_reason.is_some() {
            self.finish_reason.clone_from(&other.finish_reason);
        }
        if other.usage.is_some() {
            self.usage.clone_from(&other.usage);
        }
    }
}

/// Partial tool call received during streaming.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCallChunk {
    /// Index of this tool call in the list.
    pub index: usize,
    /// Tool call identifier, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool name, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Partial JSON arguments string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

impl ToolCallChunk {
    /// Merges another chunk into this one.
    fn merge(&mut self, other: &Self) {
        if other.id.is_some() {
            self.id.clone_from(&other.id);
        }
        if other.name.is_some() {
            self.name.clone_from(&other.name);
        }
        if let Some(ref other_args) = other.arguments {
            match self.arguments {
                Some(ref mut args) => args.push_str(other_args),
                None => self.arguments = Some(other_args.clone()),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_chunk_merge_content() {
        let mut chunk1 = ChatChunk {
            delta_content: Some("Hello".into()),
            ..Default::default()
        };
        let chunk2 = ChatChunk {
            delta_content: Some(" World".into()),
            ..Default::default()
        };
        chunk1.merge(&chunk2);
        assert_eq!(chunk1.delta_content.as_deref(), Some("Hello World"));
    }

    #[test]
    fn test_chat_chunk_merge_content_none_then_some() {
        let mut chunk1 = ChatChunk::default();
        let chunk2 = ChatChunk {
            delta_content: Some("Hello".into()),
            ..Default::default()
        };
        chunk1.merge(&chunk2);
        assert_eq!(chunk1.delta_content.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_chat_chunk_merge_tool_calls_by_index() {
        let mut chunk1 = ChatChunk {
            delta_tool_calls: vec![ToolCallChunk {
                index: 0,
                id: Some("tc_1".into()),
                name: Some("search".into()),
                arguments: Some("{\"q\":".into()),
            }],
            ..Default::default()
        };
        let chunk2 = ChatChunk {
            delta_tool_calls: vec![ToolCallChunk {
                index: 0,
                id: None,
                name: None,
                arguments: Some("\"rust\"}".into()),
            }],
            ..Default::default()
        };
        chunk1.merge(&chunk2);
        assert_eq!(chunk1.delta_tool_calls.len(), 1);
        assert_eq!(
            chunk1.delta_tool_calls[0].arguments.as_deref(),
            Some("{\"q\":\"rust\"}")
        );
    }

    #[test]
    fn test_chat_chunk_merge_finish_reason() {
        let mut chunk1 = ChatChunk::default();
        let chunk2 = ChatChunk {
            finish_reason: Some("stop".into()),
            ..Default::default()
        };
        chunk1.merge(&chunk2);
        assert_eq!(chunk1.finish_reason.as_deref(), Some("stop"));
    }
}

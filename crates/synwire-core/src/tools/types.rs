//! Tool-related type definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema describing a tool's interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's parameters.
    pub parameters: Value,
}

/// Output from a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Text result shown to the model.
    pub content: String,
    /// Rich output not sent to model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<Value>,
}

/// Result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
#[non_exhaustive]
pub enum ToolResult {
    /// Tool executed successfully.
    #[serde(rename = "success")]
    Success {
        /// The result content.
        content: Value,
    },
    /// Tool execution failed.
    #[serde(rename = "error")]
    Error {
        /// Error message.
        message: String,
    },
    /// Retry -- sent back to model for self-correction.
    #[serde(rename = "retry")]
    Retry {
        /// Retry message.
        message: String,
    },
}

/// Content types that tools can produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "content_type")]
#[non_exhaustive]
pub enum ToolContentType {
    /// Plain text.
    #[serde(rename = "text")]
    Text,
    /// JSON value.
    #[serde(rename = "json")]
    Json {
        /// The JSON value.
        value: Value,
    },
}

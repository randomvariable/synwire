//! Tool-related type definitions.

use std::time::Duration;

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

/// Binary result from tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryResult {
    /// Binary data. Serializes as a JSON array of unsigned integers (0–255).
    pub bytes: Vec<u8>,
    /// MIME type.
    pub mime_type: String,
}

/// Tool result status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum ToolResultStatus {
    /// Tool completed successfully.
    #[default]
    Success,
    /// Tool failed with error.
    Failure,
    /// Tool invocation rejected by hook.
    Rejected,
    /// Tool invocation denied by permission.
    Denied,
}

/// Content types that tools can produce.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolContentType {
    /// Plain text.
    #[default]
    Text,
    /// Structured JSON.
    Json,
    /// Image (binary).
    Image,
    /// File (binary or structured).
    File,
}

/// Category of a tool indicating its origin and deployment context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolCategory {
    /// Built-in framework tool.
    Builtin,
    /// User-defined custom tool.
    #[default]
    Custom,
    /// Tool sourced from an MCP server.
    Mcp,
    /// Remote tool (non-MCP API).
    Remote,
    /// A compiled workflow graph exposed as a tool.
    WorkflowAsTool,
}

/// Intended operation type of a tool, used for permission UIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolKind {
    /// Tool reads data without side effects.
    Read,
    /// Tool modifies data.
    Edit,
    /// Tool searches or queries.
    Search,
    /// Tool executes commands or runs code.
    Execute,
    /// Uncategorised.
    #[default]
    Other,
}

/// Behaviour when a tool invocation exceeds its configured timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TimeoutBehavior {
    /// Return a `ToolError::Timeout` to the caller.
    #[default]
    ReturnError,
    /// Propagate the timeout as a model-visible exception.
    RaiseException,
}

/// Per-tool operational configuration.
///
/// All fields are optional; unset fields use framework defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolConfig {
    /// Maximum time allowed for a single invocation.
    ///
    /// Default: no limit.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "humantime_serde::option"
    )]
    pub timeout: Option<Duration>,

    /// Behaviour when the timeout elapses.
    #[serde(default)]
    pub timeout_behavior: TimeoutBehavior,

    /// Whether the tool is available for invocation.
    ///
    /// Disabled tools are excluded from schema discovery.
    #[serde(default = "default_true")]
    pub is_enabled: bool,

    /// Maximum number of times this tool may be called in a session.
    ///
    /// `None` means unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_usage_count: Option<u32>,

    /// Maximum byte size of a tool result before truncation.
    ///
    /// Default: 100 KiB (`102_400`).
    #[serde(default = "default_max_result_size")]
    pub max_result_size: usize,
}

const fn default_true() -> bool {
    true
}

const fn default_max_result_size() -> usize {
    100 * 1_024 // 100 KiB
}

/// Output from a tool invocation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Text result shown to the model.
    pub content: String,
    /// Rich output not sent to model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<Value>,
    /// Binary results (bytes + MIME type).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binary_results: Vec<BinaryResult>,
    /// Tool result status.
    #[serde(default = "default_tool_status")]
    pub status: ToolResultStatus,
    /// Tool telemetry metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<Value>,
    /// Content type of the primary output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<ToolContentType>,
}

const fn default_tool_status() -> ToolResultStatus {
    ToolResultStatus::Success
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

/// Tool annotations for safety and behavior hints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolAnnotations {
    /// Tool only reads, no side effects.
    #[serde(default)]
    pub read_only: bool,
    /// Tool may cause data loss.
    #[serde(default)]
    pub destructive: bool,
    /// Tool accesses external resources.
    #[serde(default)]
    pub open_world: bool,
}

//! Streaming events for agents.

use crate::agents::usage::Usage;
use crate::tools::ToolOutput;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent streaming event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum AgentEvent {
    /// Text delta (streaming).
    #[serde(rename = "text_delta")]
    TextDelta {
        /// Content chunk.
        content: String,
    },

    /// Tool call started.
    #[serde(rename = "tool_call_start")]
    ToolCallStart {
        /// Tool call ID.
        id: String,
        /// Tool name.
        name: String,
    },

    /// Tool call arguments delta (streaming).
    #[serde(rename = "tool_call_delta")]
    ToolCallDelta {
        /// Tool call ID.
        id: String,
        /// Arguments delta.
        arguments_delta: String,
    },

    /// Tool call ended.
    #[serde(rename = "tool_call_end")]
    ToolCallEnd {
        /// Tool call ID.
        id: String,
    },

    /// Tool execution result.
    #[serde(rename = "tool_result")]
    ToolResult {
        /// Tool call ID.
        id: String,
        /// Tool output.
        output: ToolOutput,
    },

    /// Tool execution progress.
    #[serde(rename = "tool_progress")]
    ToolProgress {
        /// Tool call ID.
        id: String,
        /// Progress message.
        message: String,
        /// Progress percentage (0.0-1.0).
        progress_pct: Option<f32>,
    },

    /// State update patch.
    #[serde(rename = "state_update")]
    StateUpdate {
        /// JSON patch for state.
        patch: Value,
    },

    /// Directive emitted by agent.
    #[serde(rename = "directive_emitted")]
    DirectiveEmitted {
        /// Directive (serialized as Value to avoid circular dependency).
        directive: Value,
    },

    /// Status update.
    #[serde(rename = "status_update")]
    StatusUpdate {
        /// Status message.
        status: String,
        /// Progress percentage (0.0-1.0).
        progress_pct: Option<f32>,
    },

    /// Usage statistics update.
    #[serde(rename = "usage_update")]
    UsageUpdate {
        /// Token usage.
        usage: Usage,
    },

    /// Rate limit information.
    #[serde(rename = "rate_limit_info")]
    RateLimitInfo {
        /// Utilization percentage (0.0-1.0).
        utilization_pct: f32,
        /// Reset timestamp (Unix seconds).
        reset_at: i64,
        /// Whether request was allowed.
        allowed: bool,
    },

    /// Task notification.
    #[serde(rename = "task_notification")]
    TaskNotification {
        /// Task ID.
        task_id: String,
        /// Event kind.
        kind: TaskEventKind,
        /// Event payload.
        payload: Value,
    },

    /// Prompt suggestion.
    #[serde(rename = "prompt_suggestion")]
    PromptSuggestion {
        /// Suggested prompts.
        suggestions: Vec<String>,
    },

    /// Turn completed.
    #[serde(rename = "turn_complete")]
    TurnComplete {
        /// Termination reason.
        reason: TerminationReason,
    },

    /// Error occurred.
    #[serde(rename = "error")]
    Error {
        /// Error message.
        message: String,
    },
}

impl AgentEvent {
    /// Returns whether this event signals the final response.
    #[must_use]
    pub const fn is_final_response(&self) -> bool {
        matches!(self, Self::TurnComplete { .. } | Self::Error { .. })
    }
}

/// Task event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TaskEventKind {
    /// Task started.
    #[serde(rename = "started")]
    Started,
    /// Task progressed.
    #[serde(rename = "progress")]
    Progress,
    /// Task completed.
    #[serde(rename = "completed")]
    Completed,
    /// Task failed.
    #[serde(rename = "failed")]
    Failed,
}

/// Termination reason for turn completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TerminationReason {
    /// Agent finished normally.
    #[serde(rename = "complete")]
    Complete,
    /// Turn limit reached.
    #[serde(rename = "max_turns_exceeded")]
    MaxTurnsExceeded,
    /// Cost limit reached.
    #[serde(rename = "budget_exceeded")]
    BudgetExceeded,
    /// Graceful stop requested.
    #[serde(rename = "stopped")]
    Stopped,
    /// Force stop.
    #[serde(rename = "aborted")]
    Aborted,
    /// Terminated due to error.
    #[serde(rename = "error")]
    Error,
}

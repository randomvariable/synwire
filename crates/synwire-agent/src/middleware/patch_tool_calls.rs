//! Patch tool calls middleware — fixes dangling tool call references.

use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::agents::middleware::{Middleware, MiddlewareInput, MiddlewareResult};

/// Middleware that detects and patches dangling tool call messages in the
/// conversation history.
///
/// A "dangling" tool call occurs when a `tool_call` message references a
/// `tool_call_id` that has no corresponding `tool_result` message.
#[derive(Debug, Default)]
pub struct PatchToolCallsMiddleware;

impl Middleware for PatchToolCallsMiddleware {
    fn name(&self) -> &'static str {
        "patch_tool_calls"
    }

    fn process(
        &self,
        mut input: MiddlewareInput,
    ) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
        Box::pin(async move {
            // Collect tool_call_ids from the conversation.
            let mut call_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut result_ids: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            for msg in &input.messages {
                if let Some(calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                    for call in calls {
                        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
                            let _ = call_ids.insert(id.to_string());
                        }
                    }
                }
                if let Some(id) = msg.get("tool_call_id").and_then(|v| v.as_str()) {
                    let _ = result_ids.insert(id.to_string());
                }
            }

            // Find dangling calls (calls with no result).
            let dangling: Vec<String> = call_ids.difference(&result_ids).cloned().collect();
            if !dangling.is_empty() {
                tracing::debug!(count = dangling.len(), "Patching dangling tool calls");
                // Inject synthetic tool result messages for each dangling call.
                for id in &dangling {
                    input.messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": id,
                        "content": "Tool call interrupted. Please retry if needed.",
                    }));
                }
            }

            Ok(MiddlewareResult::Continue(input))
        })
    }
}

//! Prompt caching middleware — marks messages for provider-side caching.

use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::agents::middleware::{Middleware, MiddlewareInput, MiddlewareResult};

/// Middleware that adds cache control hints to the last user message.
///
/// Injects `cache_control: { type: "ephemeral" }` onto the last user message
/// so providers like Anthropic can cache it at that breakpoint.
#[derive(Debug, Default)]
pub struct PromptCachingMiddleware;

impl Middleware for PromptCachingMiddleware {
    fn name(&self) -> &'static str {
        "prompt_caching"
    }

    fn process(
        &self,
        mut input: MiddlewareInput,
    ) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
        Box::pin(async move {
            // Mark the last user message for caching.
            if let Some(last) = input
                .messages
                .iter_mut()
                .rev()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
            {
                if let Some(obj) = last.as_object_mut() {
                    let _ = obj.insert(
                        "cache_control".to_string(),
                        serde_json::json!({ "type": "ephemeral" }),
                    );
                }
            }
            Ok(MiddlewareResult::Continue(input))
        })
    }
}

//! Summarisation middleware — compacts conversation history when thresholds are hit.

use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::agents::middleware::{Middleware, MiddlewareInput, MiddlewareResult};

/// Configuration thresholds for triggering summarisation.
#[derive(Debug, Clone)]
pub struct SummarisationThresholds {
    /// Trigger when message count exceeds this value.
    pub max_messages: Option<usize>,
    /// Trigger when total token count exceeds this value (approximate).
    pub max_tokens: Option<usize>,
    /// Trigger when context utilisation percentage exceeds this value (0.0–1.0).
    pub max_context_utilisation: Option<f32>,
}

impl Default for SummarisationThresholds {
    fn default() -> Self {
        Self {
            max_messages: Some(50),
            max_tokens: Some(80_000),
            max_context_utilisation: Some(0.8),
        }
    }
}

/// Middleware that summarises conversation history when thresholds are exceeded.
#[derive(Debug)]
pub struct SummarisationMiddleware {
    thresholds: SummarisationThresholds,
}

impl SummarisationMiddleware {
    /// Create a new summarisation middleware with custom thresholds.
    #[must_use]
    pub const fn new(thresholds: SummarisationThresholds) -> Self {
        Self { thresholds }
    }
}

impl Default for SummarisationMiddleware {
    fn default() -> Self {
        Self::new(SummarisationThresholds::default())
    }
}

impl Middleware for SummarisationMiddleware {
    fn name(&self) -> &'static str {
        "summarisation"
    }

    fn process(
        &self,
        input: MiddlewareInput,
    ) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
        Box::pin(async move {
            let should_summarise = self
                .thresholds
                .max_messages
                .is_some_and(|max| input.messages.len() > max);

            if should_summarise {
                tracing::debug!(
                    messages = input.messages.len(),
                    "Summarisation threshold exceeded"
                );
                // In a real implementation, this would call the LLM to summarise.
                // For now we emit a status update via context metadata.
                let mut ctx = input.context.clone();
                if let Some(obj) = ctx.as_object_mut() {
                    let _ =
                        obj.insert("summarisation_pending".to_string(), serde_json::json!(true));
                }
                return Ok(MiddlewareResult::Continue(MiddlewareInput {
                    messages: input.messages,
                    context: ctx,
                }));
            }

            Ok(MiddlewareResult::Continue(input))
        })
    }
}

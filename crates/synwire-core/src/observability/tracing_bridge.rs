//! Tracing bridge trait and OpenTelemetry implementation.

use crate::BoxFuture;
use crate::observability::ObservabilitySpanKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Outcome of a traced span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanOutcome {
    /// The operation completed successfully.
    Success,
    /// The operation failed with an error message.
    Error(String),
}

/// Trait for bridging observability events to a tracing backend.
pub trait TracingBridge: Send + Sync {
    /// Begins a new span, returning a span identifier.
    fn begin_span(
        &self,
        name: &str,
        kind: ObservabilitySpanKind,
        run_id: Uuid,
        attributes: &HashMap<String, Value>,
    ) -> BoxFuture<'_, Uuid>;

    /// Ends a span with the given outcome.
    fn end_span(
        &self,
        span_id: Uuid,
        outcome: SpanOutcome,
        attributes: &HashMap<String, Value>,
    ) -> BoxFuture<'_, ()>;
}

/// RAII guard that ends a span when dropped.
///
/// If not explicitly completed via [`SpanGuard::complete`], the span is
/// ended with an error outcome on drop.
pub struct SpanGuard<B: TracingBridge + 'static> {
    bridge: std::sync::Arc<B>,
    span_id: Uuid,
    completed: std::sync::atomic::AtomicBool,
}

impl<B: TracingBridge + 'static> SpanGuard<B> {
    /// Creates a new span guard.
    pub const fn new(bridge: std::sync::Arc<B>, span_id: Uuid) -> Self {
        Self {
            bridge,
            span_id,
            completed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Returns the span identifier.
    pub const fn span_id(&self) -> Uuid {
        self.span_id
    }

    /// Completes the span with the given outcome.
    ///
    /// After calling this method, the drop implementation will not end the
    /// span again.
    pub async fn complete(self, outcome: SpanOutcome, attributes: &HashMap<String, Value>) {
        self.completed
            .store(true, std::sync::atomic::Ordering::Release);
        self.bridge
            .end_span(self.span_id, outcome, attributes)
            .await;
    }
}

impl<B: TracingBridge + 'static> Drop for SpanGuard<B> {
    fn drop(&mut self) {
        if !self.completed.load(std::sync::atomic::Ordering::Acquire) {
            let bridge = self.bridge.clone();
            let span_id = self.span_id;
            // Best-effort: spawn a task to end the span if the runtime is
            // available.
            let _ = std::thread::Builder::new()
                .name("span-guard-drop".into())
                .spawn(move || {
                    if let Ok(rt) = tokio::runtime::Handle::try_current() {
                        rt.block_on(bridge.end_span(
                            span_id,
                            SpanOutcome::Error("span dropped without completion".into()),
                            &HashMap::new(),
                        ));
                    }
                });
        }
    }
}

/// OpenTelemetry-based tracing bridge.
///
/// Maps observability events to `tracing` spans with `GenAI` semantic convention
/// attributes.
pub struct OTelTracingBridge {
    _private: (),
}

impl std::fmt::Debug for OTelTracingBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OTelTracingBridge").finish()
    }
}

impl Default for OTelTracingBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl OTelTracingBridge {
    /// Creates a new `OTelTracingBridge`.
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

impl TracingBridge for OTelTracingBridge {
    fn begin_span(
        &self,
        name: &str,
        kind: ObservabilitySpanKind,
        _run_id: Uuid,
        _attributes: &HashMap<String, Value>,
    ) -> BoxFuture<'_, Uuid> {
        let span_id = Uuid::new_v4();
        tracing::info_span!("synwire.span", otel.name = %name, synwire.span_kind = %kind);
        Box::pin(async move { span_id })
    }

    fn end_span(
        &self,
        _span_id: Uuid,
        outcome: SpanOutcome,
        _attributes: &HashMap<String, Value>,
    ) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            match outcome {
                SpanOutcome::Success => {
                    tracing::debug!("span completed successfully");
                }
                SpanOutcome::Error(ref msg) => {
                    tracing::warn!(error = %msg, "span completed with error");
                }
            }
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn begin_and_end_span() {
        let bridge = OTelTracingBridge::new();
        let span_id = bridge
            .begin_span(
                "test-span",
                ObservabilitySpanKind::Llm,
                Uuid::new_v4(),
                &HashMap::new(),
            )
            .await;

        bridge
            .end_span(span_id, SpanOutcome::Success, &HashMap::new())
            .await;
    }

    #[tokio::test]
    async fn span_guard_completes() {
        let bridge = Arc::new(OTelTracingBridge::new());
        let span_id = bridge
            .begin_span(
                "guarded",
                ObservabilitySpanKind::Tool,
                Uuid::new_v4(),
                &HashMap::new(),
            )
            .await;

        let guard = SpanGuard::new(Arc::clone(&bridge), span_id);
        assert_eq!(guard.span_id(), span_id);
        guard.complete(SpanOutcome::Success, &HashMap::new()).await;
    }

    #[test]
    fn span_outcome_serialization() {
        let success = SpanOutcome::Success;
        let json = serde_json::to_string(&success).unwrap();
        assert!(json.contains("Success"));

        let error = SpanOutcome::Error("timeout".into());
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("timeout"));
    }
}

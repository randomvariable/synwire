//! Callback handler that publishes events to an [`EventBus`].

use crate::BoxFuture;
use crate::callbacks::CallbackHandler;
use crate::messages::Message;
use crate::observability::content_filter::TraceContentFilter;
use crate::observability::event_bus::{EventBus, EventBusEvent, EventKind};
use crate::observability::span_kind::ObservabilitySpanKind;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// A [`CallbackHandler`] that adapts callback events into [`EventBusEvent`]s
/// and publishes them on an [`EventBus`].
///
/// Uses a [`TraceContentFilter`] to control what content is included in event
/// payloads. When content is filtered out, the payload field is omitted.
pub struct TracingCallbackHandler<B: EventBus> {
    bus: Arc<B>,
    content_filter: TraceContentFilter,
}

impl<B: EventBus> std::fmt::Debug for TracingCallbackHandler<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracingCallbackHandler")
            .field("content_filter", &self.content_filter)
            .finish_non_exhaustive()
    }
}

impl<B: EventBus> TracingCallbackHandler<B> {
    /// Creates a new `TracingCallbackHandler` with the given event bus and
    /// content filter.
    pub const fn new(bus: Arc<B>, content_filter: TraceContentFilter) -> Self {
        Self {
            bus,
            content_filter,
        }
    }

    /// Creates a new `TracingCallbackHandler` with default (include-all)
    /// content filter.
    pub fn with_default_filter(bus: Arc<B>) -> Self {
        Self {
            bus,
            content_filter: TraceContentFilter::default(),
        }
    }

    /// Publishes an event to the bus.
    fn publish_event(
        &self,
        kind: EventKind,
        span_kind: ObservabilitySpanKind,
        name: String,
        payload: Option<Value>,
    ) -> BoxFuture<'_, ()> {
        let event = EventBusEvent {
            id: Uuid::new_v4(),
            kind,
            span_kind,
            name,
            run_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            payload,
        };
        self.bus.publish(event)
    }
}

impl<B: EventBus + 'static> CallbackHandler for TracingCallbackHandler<B> {
    fn on_llm_start<'a>(
        &'a self,
        model_type: &'a str,
        messages: &'a [Message],
    ) -> BoxFuture<'a, ()> {
        let payload = if self.content_filter.include_input_messages {
            let serialized = serde_json::to_value(messages).ok();
            serialized.map(|v| {
                if let Some(max) = self.content_filter.max_content_length {
                    let s = v.to_string();
                    if s.len() > max {
                        Value::String(self.content_filter.truncate(&s).to_owned())
                    } else {
                        v
                    }
                } else {
                    v
                }
            })
        } else {
            None
        };

        self.publish_event(
            EventKind::SpanStart,
            ObservabilitySpanKind::Llm,
            model_type.to_owned(),
            payload,
        )
    }

    fn on_llm_end<'a>(&'a self, response: &'a Value) -> BoxFuture<'a, ()> {
        let payload = if self.content_filter.include_output_messages {
            Some(response.clone())
        } else {
            None
        };

        self.publish_event(
            EventKind::SpanEnd,
            ObservabilitySpanKind::Llm,
            "llm".to_owned(),
            payload,
        )
    }

    fn on_tool_start<'a>(&'a self, tool_name: &'a str, input: &'a Value) -> BoxFuture<'a, ()> {
        let payload = if self.content_filter.include_tool_arguments {
            Some(input.clone())
        } else {
            None
        };

        self.publish_event(
            EventKind::SpanStart,
            ObservabilitySpanKind::Tool,
            tool_name.to_owned(),
            payload,
        )
    }

    fn on_tool_end<'a>(&'a self, tool_name: &'a str, output: &'a str) -> BoxFuture<'a, ()> {
        let payload = if self.content_filter.include_tool_results {
            let truncated = self.content_filter.truncate(output);
            Some(Value::String(truncated.to_owned()))
        } else {
            None
        };

        self.publish_event(
            EventKind::SpanEnd,
            ObservabilitySpanKind::Tool,
            tool_name.to_owned(),
            payload,
        )
    }

    fn on_tool_error<'a>(&'a self, tool_name: &'a str, error: &'a str) -> BoxFuture<'a, ()> {
        self.publish_event(
            EventKind::Error,
            ObservabilitySpanKind::Tool,
            tool_name.to_owned(),
            Some(Value::String(error.to_owned())),
        )
    }

    fn on_retry<'a>(&'a self, attempt: u32, error: &'a str) -> BoxFuture<'a, ()> {
        let payload = serde_json::json!({
            "attempt": attempt,
            "error": error,
        });

        self.publish_event(
            EventKind::Retry,
            ObservabilitySpanKind::Chain,
            "retry".to_owned(),
            Some(payload),
        )
    }

    fn on_chain_start<'a>(&'a self, chain_name: &'a str) -> BoxFuture<'a, ()> {
        self.publish_event(
            EventKind::SpanStart,
            ObservabilitySpanKind::Chain,
            chain_name.to_owned(),
            None,
        )
    }

    fn on_chain_end<'a>(&'a self, output: &'a Value) -> BoxFuture<'a, ()> {
        let payload = if self.content_filter.include_output_messages {
            Some(output.clone())
        } else {
            None
        };

        self.publish_event(
            EventKind::SpanEnd,
            ObservabilitySpanKind::Chain,
            "chain".to_owned(),
            payload,
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::significant_drop_tightening)]
mod tests {
    use super::*;
    use crate::observability::event_bus::{EventFilter, InMemoryEventBus};

    #[tokio::test]
    async fn publishes_llm_start_with_filter() {
        let bus = Arc::new(InMemoryEventBus::new());
        let handler = TracingCallbackHandler::with_default_filter(Arc::clone(&bus));
        let mut rx = bus.subscribe(EventFilter::all()).await;

        handler.on_llm_start("gpt-4", &[]).await;

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.kind, EventKind::SpanStart);
        assert_eq!(event.span_kind, ObservabilitySpanKind::Llm);
        assert_eq!(event.name, "gpt-4");
    }

    #[tokio::test]
    async fn redacts_input_when_filtered() {
        let bus = Arc::new(InMemoryEventBus::new());
        let filter = TraceContentFilter::builder()
            .include_input_messages(false)
            .build();
        let handler = TracingCallbackHandler::new(Arc::clone(&bus), filter);
        let mut rx = bus.subscribe(EventFilter::all()).await;

        handler.on_llm_start("gpt-4", &[]).await;

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(event.payload.is_none(), "input should be redacted");
    }

    #[tokio::test]
    async fn redacts_tool_arguments_when_filtered() {
        let bus = Arc::new(InMemoryEventBus::new());
        let filter = TraceContentFilter::builder()
            .include_tool_arguments(false)
            .build();
        let handler = TracingCallbackHandler::new(Arc::clone(&bus), filter);
        let mut rx = bus.subscribe(EventFilter::all()).await;

        handler
            .on_tool_start("search", &serde_json::json!({"query": "secret"}))
            .await;

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(event.payload.is_none(), "tool arguments should be redacted");
    }

    #[tokio::test]
    async fn publishes_tool_error() {
        let bus = Arc::new(InMemoryEventBus::new());
        let handler = TracingCallbackHandler::with_default_filter(Arc::clone(&bus));
        let mut rx = bus.subscribe(EventFilter::all()).await;

        handler.on_tool_error("search", "connection refused").await;

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.kind, EventKind::Error);
        assert_eq!(event.span_kind, ObservabilitySpanKind::Tool);
    }

    #[tokio::test]
    async fn secret_value_serializes_as_null() {
        use crate::credentials::SecretValue;
        let secret = SecretValue::new("api-key-12345");
        let json = serde_json::to_value(&secret).unwrap();
        assert!(json.is_null(), "SecretValue must serialize as null");
    }

    #[tokio::test]
    async fn truncates_tool_output() {
        let bus = Arc::new(InMemoryEventBus::new());
        let filter = TraceContentFilter::builder()
            .max_content_length(Some(10))
            .build();
        let handler = TracingCallbackHandler::new(Arc::clone(&bus), filter);
        let mut rx = bus.subscribe(EventFilter::all()).await;

        handler
            .on_tool_end("search", "a very long result that exceeds the limit")
            .await;

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        if let Some(Value::String(s)) = &event.payload {
            assert!(s.len() <= 10, "output should be truncated");
        }
    }
}

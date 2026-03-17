//! Event bus for publishing and subscribing to observability events.

use crate::BoxFuture;
use crate::observability::ObservabilitySpanKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// The kind of event published on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventKind {
    /// A span has started.
    SpanStart,
    /// A span has ended.
    SpanEnd,
    /// A token was generated (streaming).
    Token,
    /// An error occurred.
    Error,
    /// A retry was attempted.
    Retry,
}

/// An event published on the [`EventBus`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusEvent {
    /// Unique event identifier.
    pub id: Uuid,
    /// The kind of event.
    pub kind: EventKind,
    /// The span kind this event relates to.
    pub span_kind: ObservabilitySpanKind,
    /// Name of the operation (e.g., model name, tool name).
    pub name: String,
    /// Run identifier for correlation.
    pub run_id: Uuid,
    /// Timestamp of the event.
    pub timestamp: DateTime<Utc>,
    /// Arbitrary payload data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

/// Filter predicate for event subscriptions.
pub struct EventFilter {
    /// If set, only events matching this kind are delivered.
    pub kind: Option<EventKind>,
    /// If set, only events matching this span kind are delivered.
    pub span_kind: Option<ObservabilitySpanKind>,
}

impl EventFilter {
    /// Creates a filter that accepts all events.
    pub const fn all() -> Self {
        Self {
            kind: None,
            span_kind: None,
        }
    }

    /// Returns `true` if the given event matches this filter.
    pub fn matches(&self, event: &EventBusEvent) -> bool {
        if let Some(kind) = self.kind
            && event.kind != kind
        {
            return false;
        }
        if let Some(span_kind) = self.span_kind
            && event.span_kind != span_kind
        {
            return false;
        }
        true
    }
}

/// Trait for an event bus that supports publish/subscribe of observability
/// events.
pub trait EventBus: Send + Sync {
    /// Publishes an event to all subscribers.
    fn publish(&self, event: EventBusEvent) -> BoxFuture<'_, ()>;

    /// Subscribes to events, returning a receiver.
    fn subscribe(
        &self,
        filter: EventFilter,
    ) -> BoxFuture<'_, tokio::sync::mpsc::Receiver<EventBusEvent>>;
}

/// Default broadcast capacity for [`InMemoryEventBus`].
const DEFAULT_CAPACITY: usize = 1024;

/// In-memory event bus backed by `tokio::sync::broadcast`.
///
/// Subscribers receive events through an MPSC channel that is fed from a
/// broadcast receiver with optional filtering.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "event-bus")]
/// # {
/// use synwire_core::observability::{
///     EventBus, EventBusEvent, EventFilter, EventKind, InMemoryEventBus,
///     ObservabilitySpanKind,
/// };
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// # tokio_test::block_on(async {
/// let bus = InMemoryEventBus::new();
/// let mut rx = bus.subscribe(EventFilter::all()).await;
///
/// let event = EventBusEvent {
///     id: Uuid::new_v4(),
///     kind: EventKind::SpanStart,
///     span_kind: ObservabilitySpanKind::Llm,
///     name: "gpt-4".into(),
///     run_id: Uuid::new_v4(),
///     timestamp: Utc::now(),
///     payload: None,
/// };
/// bus.publish(event.clone()).await;
/// let received = rx.recv().await;
/// assert!(received.is_some());
/// # });
/// # }
/// ```
pub struct InMemoryEventBus {
    sender: tokio::sync::broadcast::Sender<EventBusEvent>,
}

impl std::fmt::Debug for InMemoryEventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryEventBus").finish_non_exhaustive()
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryEventBus {
    /// Creates a new `InMemoryEventBus` with the default capacity (1024).
    pub fn new() -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(DEFAULT_CAPACITY);
        Self { sender }
    }

    /// Creates a new `InMemoryEventBus` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }
}

impl EventBus for InMemoryEventBus {
    fn publish(&self, event: EventBusEvent) -> BoxFuture<'_, ()> {
        // Ignore send errors (no subscribers).
        let _ = self.sender.send(event);
        Box::pin(async {})
    }

    fn subscribe(
        &self,
        filter: EventFilter,
    ) -> BoxFuture<'_, tokio::sync::mpsc::Receiver<EventBusEvent>> {
        let mut broadcast_rx = self.sender.subscribe();
        let (tx, rx) = tokio::sync::mpsc::channel(DEFAULT_CAPACITY);

        let _handle = tokio::spawn(async move {
            loop {
                match broadcast_rx.recv().await {
                    Ok(event) => {
                        if filter.matches(&event) && tx.send(event).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
        });

        Box::pin(async { rx })
    }
}

/// Allow `InMemoryEventBus` to be shared across threads via `Arc`.
impl EventBus for Arc<InMemoryEventBus> {
    fn publish(&self, event: EventBusEvent) -> BoxFuture<'_, ()> {
        (**self).publish(event)
    }

    fn subscribe(
        &self,
        filter: EventFilter,
    ) -> BoxFuture<'_, tokio::sync::mpsc::Receiver<EventBusEvent>> {
        (**self).subscribe(filter)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::significant_drop_tightening)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_event(kind: EventKind, span_kind: ObservabilitySpanKind) -> EventBusEvent {
        EventBusEvent {
            id: Uuid::new_v4(),
            kind,
            span_kind,
            name: "test".into(),
            run_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            payload: None,
        }
    }

    #[tokio::test]
    async fn subscribe_and_publish() {
        let bus = InMemoryEventBus::new();
        let mut rx = bus.subscribe(EventFilter::all()).await;

        let event = make_event(EventKind::SpanStart, ObservabilitySpanKind::Llm);
        let event_id = event.id;
        bus.publish(event).await;

        let received = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received.id, event_id);
    }

    #[tokio::test]
    async fn filter_by_kind() {
        let bus = InMemoryEventBus::new();
        let mut rx = bus
            .subscribe(EventFilter {
                kind: Some(EventKind::Error),
                span_kind: None,
            })
            .await;

        // Publish a non-matching event
        bus.publish(make_event(EventKind::SpanStart, ObservabilitySpanKind::Llm))
            .await;
        // Publish a matching event
        let error_event = make_event(EventKind::Error, ObservabilitySpanKind::Tool);
        let error_id = error_event.id;
        bus.publish(error_event).await;

        let received = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received.id, error_id);
        assert_eq!(received.kind, EventKind::Error);
    }

    #[tokio::test]
    async fn filter_by_span_kind() {
        let bus = InMemoryEventBus::new();
        let mut rx = bus
            .subscribe(EventFilter {
                kind: None,
                span_kind: Some(ObservabilitySpanKind::Tool),
            })
            .await;

        bus.publish(make_event(EventKind::SpanStart, ObservabilitySpanKind::Llm))
            .await;
        let tool_event = make_event(EventKind::SpanStart, ObservabilitySpanKind::Tool);
        let tool_id = tool_event.id;
        bus.publish(tool_event).await;

        let received = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received.id, tool_id);
    }

    #[test]
    fn event_filter_all_matches_everything() {
        let filter = EventFilter::all();
        let event = make_event(EventKind::SpanEnd, ObservabilitySpanKind::Graph);
        assert!(filter.matches(&event));
    }

    #[test]
    fn event_filter_rejects_non_matching() {
        let filter = EventFilter {
            kind: Some(EventKind::Token),
            span_kind: None,
        };
        let event = make_event(EventKind::SpanStart, ObservabilitySpanKind::Llm);
        assert!(!filter.matches(&event));
    }
}

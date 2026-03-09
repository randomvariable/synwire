# Observability Contracts

**Date**: 2026-03-09
**Branch**: `001-synwire`
**Spec refs**: FR-511–FR-556, SC-091–SC-096

Trait contracts for the observability stack: EventBus (Layer 2),
tracing integration (Layer 3), OTel mapping (Layer 4), and exporter
crates (Layer 5). Layer 1 (CallbackHandler) is defined in
[traits.md](traits.md).

## EventBus (Layer 2)

```rust
/// Internal event bus for framework lifecycle events.
/// Provides typed pub-sub for observability integration.
/// Distinct from CallbackHandler (user-facing) — EventBus is for
/// internal framework coordination and exporter integration.
pub trait EventBus: Send + Sync {
    /// Subscribe to events matching a filter.
    /// Returns a subscription handle that can be dropped to unsubscribe.
    fn subscribe(
        &self,
        filter: EventFilter,
    ) -> BoxStream<'static, EventBusEvent>;

    /// Publish an event to all matching subscribers.
    fn publish<'a>(
        &'a self,
        event: EventBusEvent,
    ) -> BoxFuture<'a, ()>;
}

/// Filter for event subscriptions.
pub struct EventFilter {
    pub event_kinds: Option<Vec<EventKind>>,  // None = all events
    pub run_ids: Option<Vec<String>>,         // None = all runs
}

/// Discriminant for EventBusEvent variants (used by EventFilter).
pub enum EventKind {
    AgentStart,
    AgentEnd,
    ModelCall,
    ModelResponse,
    ModelStreamStart,
    ToolCall,
    ToolResult,
    EmbeddingCall,
    EmbeddingResult,
    RetrieverCall,
    RetrieverResult,
    MemoryWrite,
    Handoff,
    RetryAttempt,
    FallbackTriggered,
}
```

### Default Implementation: InMemoryEventBus

```rust
/// Broadcast-channel-based EventBus using tokio::sync::broadcast.
/// Default capacity: 1024 events. Lagging subscribers receive
/// a RecvError::Lagged and skip missed events with a warning.
pub struct InMemoryEventBus {
    capacity: usize,  // default: 1024
}
```

## TracingBridge (Layer 3)

```rust
/// Bridge between EventBus events and the `tracing` crate.
/// Converts EventBusEvents into tracing spans with OTel-compatible
/// attributes. Activated by the `tracing` feature flag (FR-555).
pub trait TracingBridge: Send + Sync {
    /// Start a new span for the given event. Returns a span guard
    /// that must be held for the span's lifetime.
    fn begin_span<'a>(
        &'a self,
        event: &'a EventBusEvent,
    ) -> BoxFuture<'a, SpanGuard>;

    /// End a span associated with the given run_id.
    fn end_span<'a>(
        &'a self,
        run_id: &'a str,
        outcome: SpanOutcome,
    ) -> BoxFuture<'a, ()>;
}

/// Outcome of a span — determines OTel status code.
pub enum SpanOutcome {
    Ok,
    Error { message: String, error_type: Option<String> },
}

/// Opaque span guard. Drop to end the span.
/// Holds a tracing::span::Entered or equivalent.
pub struct SpanGuard {
    // Internal: tracing::Span + entered guard
    _inner: Box<dyn std::any::Any + Send>,
}
```

### Default Implementation: OTelTracingBridge

```rust
/// Maps EventBusEvent → tracing spans with OTel GenAI attributes.
/// Uses tracing-opentelemetry for span export.
///
/// Attribute mapping:
///   EventBusEvent::ModelCall   → gen_ai.operation.name, gen_ai.request.model, gen_ai.system
///   EventBusEvent::ModelResponse → gen_ai.usage.*, gen_ai.response.finish_reasons
///   EventBusEvent::AgentStart  → gen_ai.agent.name, gen_ai.operation.name: "invoke_agent"
///   EventBusEvent::ToolCall    → gen_ai.tool.name, gen_ai.operation.name: "execute_tool"
///   EventBusEvent::EmbeddingCall → gen_ai.operation.name: "embeddings", gen_ai.request.model
///   EventBusEvent::RetrieverCall → gen_ai.operation.name: "retrieval"
pub struct OTelTracingBridge {
    content_filter: TraceContentFilter,
}
```

## OTelAttributeMapper (Layer 4)

```rust
/// Maps framework types to OTel span attributes.
/// Used by TracingBridge and exporter crates to ensure consistent
/// attribute naming per OTel GenAI semantic conventions.
pub trait OTelAttributeMapper: Send + Sync {
    /// Map a model call event to OTel span attributes.
    fn map_model_call<'a>(
        &'a self,
        model: &'a str,
        provider: Option<&'a str>,
        operation: &'a str,
        request_params: Option<&'a HashMap<String, Value>>,
    ) -> Vec<KeyValue>;

    /// Map a model response to OTel span attributes.
    fn map_model_response<'a>(
        &'a self,
        usage: &'a UsageMetadata,
        finish_reason: Option<&'a str>,
        response_id: Option<&'a str>,
        response_model: Option<&'a str>,
    ) -> Vec<KeyValue>;

    /// Map an agent event to OTel span attributes.
    fn map_agent(
        &self,
        agent_name: &str,
        agent_id: Option<&str>,
        description: Option<&str>,
    ) -> Vec<KeyValue>;

    /// Map a tool event to OTel span attributes.
    fn map_tool(
        &self,
        tool_name: &str,
        tool_call_id: Option<&str>,
        description: Option<&str>,
    ) -> Vec<KeyValue>;

    /// Map an embedding event to OTel span attributes.
    fn map_embedding(
        &self,
        model: &str,
        input_count: usize,
    ) -> Vec<KeyValue>;

    /// Map a retrieval event to OTel span attributes.
    fn map_retrieval(
        &self,
        query: &str,
        document_count: Option<usize>,
    ) -> Vec<KeyValue>;
}
```

### Default Implementation: GenAIAttributeMapper

Implements OTel GenAI semantic conventions v1.32+. Attribute keys from
`OTelGenAIAttributes` constants (see data-model.md).

## SpanExporter (Layer 5)

Exporter crates implement the OTel `SpanExporter` trait from
`opentelemetry_sdk::export::trace`. These are NOT custom traits —
they use the standard OTel SDK interface.

### Generic OTLP Export

Traces are exported via the standard OTel OTLP exporter (gRPC or HTTP)
to any OTel-compatible backend (Jaeger, Grafana Tempo, Datadog, etc.).
No vendor-specific exporter crates are shipped — users configure the
`opentelemetry-otlp` exporter directly with their backend endpoint.

## MetricsCollector

```rust
/// Collects OTel GenAI metrics from EventBus events (FR-536).
/// Registers histogram instruments and records measurements.
pub trait MetricsCollector: Send + Sync {
    /// Record token usage for a model response.
    fn record_token_usage<'a>(
        &'a self,
        model: &'a str,
        provider: &'a str,
        operation: &'a str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> BoxFuture<'a, ()>;

    /// Record operation duration.
    fn record_operation_duration<'a>(
        &'a self,
        model: &'a str,
        provider: &'a str,
        operation: &'a str,
        duration: std::time::Duration,
        finish_reason: Option<&'a str>,
        error_type: Option<&'a str>,
    ) -> BoxFuture<'a, ()>;

    /// Record time to first token (streaming only).
    fn record_ttft<'a>(
        &'a self,
        model: &'a str,
        provider: &'a str,
        operation: &'a str,
        ttft: std::time::Duration,
    ) -> BoxFuture<'a, ()>;
}
```

## TraceContextPropagator

```rust
/// Propagates W3C Trace Context across protocol boundaries
/// (A2A, MCP, AG-UI) per FR-532–FR-535.
pub trait TraceContextPropagator: Send + Sync {
    /// Inject trace context into outgoing request headers/metadata.
    fn inject(&self, context: &TraceContext, carrier: &mut dyn TextMapCarrier);

    /// Extract trace context from incoming request headers/metadata.
    fn extract(&self, carrier: &dyn TextMapCarrier) -> Option<TraceContext>;
}

/// Carrier trait for injecting/extracting trace context.
/// Implementations exist for:
///   - HTTP headers (reqwest::header::HeaderMap)
///   - A2A Task metadata
///   - MCP request _meta field
///   - AG-UI run config headers
pub trait TextMapCarrier {
    fn get(&self, key: &str) -> Option<&str>;
    fn set(&mut self, key: &str, value: String);
    fn keys(&self) -> Vec<&str>;
}
```

## TracingCallbackHandler

```rust
/// A CallbackHandler implementation that bridges callbacks to the
/// EventBus + tracing infrastructure. This is the "Layer 1 → Layer 2"
/// adapter. Install via RunnableConfig::callbacks.
///
/// Wraps any inner CallbackHandler and additionally publishes all
/// callback events to the EventBus for observability processing.
///
/// FR-556: TracingAgentWrapper uses this internally.
pub struct TracingCallbackHandler {
    inner: Option<Box<dyn CallbackHandler>>,
    event_bus: Arc<dyn EventBus>,
    content_filter: TraceContentFilter,
    span_kind_resolver: Box<dyn Fn(&str) -> ObservabilitySpanKind + Send + Sync>,
}

impl CallbackHandler for TracingCallbackHandler {
    // All on_* methods:
    //   1. Apply content_filter to redact sensitive data
    //   2. Publish corresponding EventBusEvent
    //   3. Forward to inner.on_*() if inner is Some
}
```

## Feature Flag Scope (FR-555)

All observability types and traits live behind `#[cfg(feature = "tracing")]`:

| Crate | Feature | What it gates |
|-------|---------|---------------|
| `synwire-core` | `tracing` | EventBus, TracingBridge, OTelAttributeMapper, MetricsCollector, TracingCallbackHandler, TraceContextPropagator |
| `synwire-core` | (always) | CallbackHandler, TraceContentFilter, ObservabilitySpanKind, TraceContext |

Core callback types (`CallbackHandler`, `TraceContentFilter`,
`ObservabilitySpanKind`) are always available — they have zero external
dependencies. The `tracing` feature adds the OTel bridge layer and
requires `tracing`, `tracing-opentelemetry`, `opentelemetry`,
`opentelemetry-sdk` dependencies.

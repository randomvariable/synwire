//! Observability stack for the Synwire framework.
//!
//! This module provides tracing, metrics, and event-bus abstractions for
//! monitoring AI agent execution. All observability features are opt-in via
//! Cargo feature flags:
//!
//! - `event-bus`: In-memory publish/subscribe event bus (requires `tokio`).
//! - `tracing`: OpenTelemetry tracing bridge and metrics collector (requires
//!   `tracing`, `tracing-opentelemetry`, `opentelemetry`, `opentelemetry_sdk`).
//!
//! Core types such as [`ObservabilitySpanKind`], [`TraceContentFilter`], and the
//! `OTel` attribute/metric constants are always available.

mod content_filter;
mod otel_attributes;
mod otel_metrics;
mod span_kind;

pub use content_filter::TraceContentFilter;
pub use otel_attributes::gen_ai;
pub use otel_metrics::gen_ai_metrics;
pub use span_kind::ObservabilitySpanKind;

#[cfg(feature = "event-bus")]
mod event_bus;
#[cfg(feature = "event-bus")]
pub use event_bus::{EventBus, EventBusEvent, EventFilter, EventKind, InMemoryEventBus};

#[cfg(feature = "tracing")]
mod tracing_bridge;
#[cfg(feature = "tracing")]
pub use tracing_bridge::{OTelTracingBridge, SpanGuard, SpanOutcome, TracingBridge};

#[cfg(feature = "tracing")]
mod attribute_mapper;
#[cfg(feature = "tracing")]
pub use attribute_mapper::{GenAIAttributeMapper, OTelAttributeMapper};

#[cfg(feature = "tracing")]
mod metrics_collector;
#[cfg(feature = "tracing")]
pub use metrics_collector::{MetricsCollector, OTelMetricsCollector};

#[cfg(feature = "tracing")]
mod tracing_config;
#[cfg(feature = "tracing")]
pub use tracing_config::{BatchConfig, TracingConfig};

#[cfg(feature = "event-bus")]
mod tracing_callback_handler;
#[cfg(feature = "event-bus")]
pub use tracing_callback_handler::TracingCallbackHandler;

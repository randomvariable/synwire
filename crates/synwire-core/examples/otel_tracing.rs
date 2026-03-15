//! Example: OpenTelemetry tracing with Synwire observability.
//!
//! Run with:
//! ```sh
//! cargo run -p synwire-core --example otel_tracing --features tracing,event-bus
//! ```
//!
//! This example demonstrates:
//! - Creating a `TracingConfig`
//! - Setting up an `InMemoryEventBus`
//! - Using the `TracingCallbackHandler` to publish events
//! - Subscribing to events with filtering

#[cfg(all(feature = "tracing", feature = "event-bus"))]
#[allow(clippy::print_stderr)]
#[tokio::main]
async fn main() {
    use std::sync::Arc;
    use synwire_core::callbacks::CallbackHandler;
    use synwire_core::observability::{
        EventBus, EventFilter, InMemoryEventBus, ObservabilitySpanKind, TraceContentFilter,
        TracingCallbackHandler, TracingConfig,
    };

    // 1. Configure tracing (opt-in, disabled by default).
    let _config = TracingConfig::builder()
        .enabled(true)
        .service_name("otel-example".to_owned())
        .content_filter(
            TraceContentFilter::builder()
                .include_system_instructions(false)
                .max_content_length(Some(256))
                .build(),
        )
        .build();

    // 2. Create the event bus.
    let bus = Arc::new(InMemoryEventBus::new());

    // 3. Subscribe to LLM events only.
    let mut rx = bus
        .subscribe(EventFilter {
            kind: None,
            span_kind: Some(ObservabilitySpanKind::Llm),
        })
        .await;

    // 4. Create the callback handler.
    let handler = TracingCallbackHandler::with_default_filter(Arc::clone(&bus));

    // 5. Simulate LLM invocation.
    handler.on_llm_start("gpt-4", &[]).await;
    handler
        .on_llm_end(&serde_json::json!({"content": "Hello!"}))
        .await;

    // 6. Read events.
    let mut count = 0u32;
    while let Ok(Some(event)) =
        tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await
    {
        count = count.saturating_add(1);
        eprintln!(
            "[{kind:?}] {span_kind} :: {name} (id={id})",
            kind = event.kind,
            span_kind = event.span_kind,
            name = event.name,
            id = event.id,
        );
    }

    eprintln!("Received {count} events.");
}

#[cfg(not(all(feature = "tracing", feature = "event-bus")))]
#[allow(clippy::print_stderr)]
fn main() {
    eprintln!(
        "This example requires both the `tracing` and `event-bus` features.\n\
         Run with: cargo run -p synwire-core --example otel_tracing --features tracing,event-bus"
    );
}

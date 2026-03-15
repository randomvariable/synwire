//! Metrics collector trait and OpenTelemetry implementation.

use crate::BoxFuture;
use crate::observability::gen_ai_metrics;
use std::collections::HashMap;

/// Trait for collecting observability metrics.
pub trait MetricsCollector: Send + Sync {
    /// Records token usage (input and output tokens).
    fn record_token_usage(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        attributes: &HashMap<String, String>,
    ) -> BoxFuture<'_, ()>;

    /// Records operation duration in seconds.
    fn record_operation_duration(
        &self,
        duration_secs: f64,
        attributes: &HashMap<String, String>,
    ) -> BoxFuture<'_, ()>;

    /// Records time-to-first-chunk for streaming operations, in seconds.
    fn record_time_to_first_chunk(
        &self,
        duration_secs: f64,
        attributes: &HashMap<String, String>,
    ) -> BoxFuture<'_, ()>;
}

/// OpenTelemetry-based metrics collector using histogram instruments.
///
/// Creates `OTel` histograms for the `GenAI` semantic convention metrics.
pub struct OTelMetricsCollector {
    token_usage: opentelemetry::metrics::Histogram<u64>,
    operation_duration: opentelemetry::metrics::Histogram<f64>,
    time_to_first_chunk: opentelemetry::metrics::Histogram<f64>,
}

impl std::fmt::Debug for OTelMetricsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OTelMetricsCollector").finish()
    }
}

impl OTelMetricsCollector {
    /// Creates a new `OTelMetricsCollector` using the given meter.
    pub fn new(meter: &opentelemetry::metrics::Meter) -> Self {
        let token_usage = meter
            .u64_histogram(gen_ai_metrics::CLIENT_TOKEN_USAGE)
            .with_description("GenAI client token usage")
            .build();
        let operation_duration = meter
            .f64_histogram(gen_ai_metrics::CLIENT_OPERATION_DURATION)
            .with_description("GenAI client operation duration in seconds")
            .build();
        let time_to_first_chunk = meter
            .f64_histogram(gen_ai_metrics::CLIENT_OPERATION_TIME_TO_FIRST_CHUNK)
            .with_description("GenAI client time-to-first-chunk in seconds")
            .build();

        Self {
            token_usage,
            operation_duration,
            time_to_first_chunk,
        }
    }
}

/// Converts a `HashMap<String, String>` into `OTel` `KeyValue` pairs.
fn to_key_values(attrs: &HashMap<String, String>) -> Vec<opentelemetry::KeyValue> {
    attrs
        .iter()
        .map(|(k, v)| opentelemetry::KeyValue::new(k.clone(), v.clone()))
        .collect()
}

impl MetricsCollector for OTelMetricsCollector {
    fn record_token_usage(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        attributes: &HashMap<String, String>,
    ) -> BoxFuture<'_, ()> {
        let kvs = to_key_values(attributes);
        self.token_usage.record(input_tokens + output_tokens, &kvs);
        Box::pin(async {})
    }

    fn record_operation_duration(
        &self,
        duration_secs: f64,
        attributes: &HashMap<String, String>,
    ) -> BoxFuture<'_, ()> {
        let kvs = to_key_values(attributes);
        self.operation_duration.record(duration_secs, &kvs);
        Box::pin(async {})
    }

    fn record_time_to_first_chunk(
        &self,
        duration_secs: f64,
        attributes: &HashMap<String, String>,
    ) -> BoxFuture<'_, ()> {
        let kvs = to_key_values(attributes);
        self.time_to_first_chunk.record(duration_secs, &kvs);
        Box::pin(async {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_key_values_converts_correctly() {
        let mut attrs = HashMap::new();
        let _ = attrs.insert("key1".to_owned(), "val1".to_owned());
        let _ = attrs.insert("key2".to_owned(), "val2".to_owned());

        let kvs = to_key_values(&attrs);
        assert_eq!(kvs.len(), 2);
    }

    #[tokio::test]
    async fn otel_metrics_collector_records() {
        let meter = opentelemetry::global::meter("test");
        let collector = OTelMetricsCollector::new(&meter);
        let attrs = HashMap::new();

        collector.record_token_usage(100, 50, &attrs).await;
        collector.record_operation_duration(1.5, &attrs).await;
        collector.record_time_to_first_chunk(0.2, &attrs).await;
    }
}

//! OpenTelemetry `GenAI` semantic convention metric constants.
//!
//! These follow the [`OTel` `GenAI` Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/).

/// `OTel` `GenAI` metric name constants.
///
/// # Example
///
/// ```
/// use synwire_core::observability::gen_ai_metrics;
///
/// assert_eq!(gen_ai_metrics::CLIENT_TOKEN_USAGE, "gen_ai.client.token.usage");
/// ```
pub mod gen_ai_metrics {
    /// Histogram of token usage (input and output).
    pub const CLIENT_TOKEN_USAGE: &str = "gen_ai.client.token.usage";

    /// Histogram of operation duration in seconds.
    pub const CLIENT_OPERATION_DURATION: &str = "gen_ai.client.operation.duration";

    /// Histogram of time-to-first-chunk for streaming operations, in seconds.
    pub const CLIENT_OPERATION_TIME_TO_FIRST_CHUNK: &str =
        "gen_ai.client.operation.time_to_first_chunk";
}

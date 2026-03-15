//! Tracing and batch configuration types.
//!
//! These are only available when the `tracing` feature is enabled.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for batch processing of observability events.
///
/// Controls how events are batched before export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of events in a batch before flushing.
    pub max_batch_size: usize,
    /// Maximum time to wait before flushing an incomplete batch.
    #[serde(with = "duration_secs")]
    pub max_wait: Duration,
    /// Maximum number of concurrent export operations.
    pub max_concurrent_exports: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 512,
            max_wait: Duration::from_secs(5),
            max_concurrent_exports: 4,
        }
    }
}

/// Top-level tracing configuration.
///
/// Controls whether tracing is enabled, what content is captured, and batch
/// export behaviour.
///
/// # Feature gate
///
/// This type is only available when the `tracing` feature is enabled. Tracing
/// is opt-in, not a default feature, to avoid pulling in OpenTelemetry
/// dependencies unless explicitly needed.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "tracing")]
/// # {
/// use synwire_core::observability::TracingConfig;
///
/// let config = TracingConfig::builder()
///     .enabled(true)
///     .service_name("my-agent".to_owned())
///     .build();
/// assert!(config.enabled);
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Whether tracing is enabled.
    pub enabled: bool,
    /// The service name reported to the tracing backend.
    pub service_name: String,
    /// Content filter for traces.
    pub content_filter: super::TraceContentFilter,
    /// Batch export configuration.
    pub batch: BatchConfig,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: "synwire".to_owned(),
            content_filter: super::TraceContentFilter::default(),
            batch: BatchConfig::default(),
        }
    }
}

impl TracingConfig {
    /// Creates a builder for `TracingConfig`.
    pub fn builder() -> TracingConfigBuilder {
        TracingConfigBuilder::default()
    }
}

/// Builder for [`TracingConfig`].
#[derive(Debug, Default)]
pub struct TracingConfigBuilder {
    config: TracingConfig,
}

impl TracingConfigBuilder {
    /// Sets whether tracing is enabled.
    pub const fn enabled(mut self, value: bool) -> Self {
        self.config.enabled = value;
        self
    }

    /// Sets the service name.
    pub fn service_name(mut self, name: String) -> Self {
        self.config.service_name = name;
        self
    }

    /// Sets the content filter.
    pub const fn content_filter(mut self, filter: super::TraceContentFilter) -> Self {
        self.config.content_filter = filter;
        self
    }

    /// Sets the batch configuration.
    pub const fn batch(mut self, batch: BatchConfig) -> Self {
        self.config.batch = batch;
        self
    }

    /// Builds the [`TracingConfig`].
    pub fn build(self) -> TracingConfig {
        self.config
    }
}

/// Serde helper for `Duration` as seconds (f64).
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    /// Serialises a `Duration` as seconds (f64).
    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        duration.as_secs_f64().serialize(serializer)
    }

    /// Deserialises a `Duration` from seconds (f64).
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let secs = f64::deserialize(deserializer)?;
        Ok(Duration::from_secs_f64(secs))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled() {
        let config = TracingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.service_name, "synwire");
    }

    #[test]
    fn builder_overrides() {
        let config = TracingConfig::builder()
            .enabled(true)
            .service_name("test-agent".to_owned())
            .build();
        assert!(config.enabled);
        assert_eq!(config.service_name, "test-agent");
    }

    #[test]
    fn batch_config_defaults() {
        let batch = BatchConfig::default();
        assert_eq!(batch.max_batch_size, 512);
        assert_eq!(batch.max_wait, Duration::from_secs(5));
        assert_eq!(batch.max_concurrent_exports, 4);
    }

    #[test]
    fn tracing_config_serialization_roundtrip() {
        let config = TracingConfig::builder()
            .enabled(true)
            .service_name("roundtrip".to_owned())
            .build();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TracingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.service_name, config.service_name);
    }
}

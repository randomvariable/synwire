//! Runnable configuration.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Configuration passed through a runnable chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnableConfig {
    /// Tags for filtering and categorization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Metadata key-value pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
    /// Maximum concurrency for batch operations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<usize>,
    /// Name for this run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_name: Option<String>,
    /// Unique identifier for this run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<Uuid>,
    /// Configurable key-value pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configurable: Option<HashMap<String, Value>>,
    /// Callback handlers (not serialized).
    #[serde(skip)]
    pub callbacks: Option<Vec<Arc<dyn CallbackHandlerDyn>>>,
    /// Tracing configuration (only available with the `tracing` feature).
    #[cfg(feature = "tracing")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracing_config: Option<crate::observability::TracingConfig>,
}

/// Marker trait for callback handlers (placeholder until callbacks module exists).
pub trait CallbackHandlerDyn: Send + Sync + std::fmt::Debug {}

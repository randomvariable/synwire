//! Cache policy configuration for node results.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Configuration for caching node execution results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy {
    /// Whether caching is enabled.
    pub enabled: bool,
    /// Time-to-live for cached results.
    pub ttl: Duration,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl: Duration::from_secs(300),
        }
    }
}

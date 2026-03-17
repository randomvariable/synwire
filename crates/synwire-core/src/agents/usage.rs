//! Usage tracking and cost estimation.

use serde::{Deserialize, Serialize};

/// Token usage and cost tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed.
    pub input_tokens: u64,
    /// Output tokens generated.
    pub output_tokens: u64,
    /// Tokens read from cache.
    pub cache_read_tokens: u64,
    /// Tokens written to cache.
    pub cache_creation_tokens: u64,
    /// Estimated cost in USD.
    pub cost_usd: f64,
    /// Context window utilization (0.0-1.0).
    pub context_utilization_pct: f32,
}

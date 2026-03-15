//! Execution metrics for graph runs.
//!
//! Stub module for future metrics and instrumentation support.

use std::time::Duration;

/// Metrics collected during a graph execution run.
#[derive(Debug, Clone, Default)]
pub struct GraphExecutionMetrics {
    /// Total number of supersteps executed.
    pub steps: usize,
    /// Total wall-clock duration of the execution.
    pub total_duration: Duration,
    /// Per-node execution durations, keyed by node name.
    pub node_durations: Vec<(String, Duration)>,
}

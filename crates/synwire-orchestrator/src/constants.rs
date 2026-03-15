//! Well-known node names used by the graph engine.

/// The virtual start node. Edges from this node define the graph entry point.
pub const START: &str = "__start__";

/// The virtual end node. Edges to this node mark terminal states.
pub const END: &str = "__end__";

/// Default recursion limit for graph execution.
pub const DEFAULT_RECURSION_LIMIT: usize = 25;

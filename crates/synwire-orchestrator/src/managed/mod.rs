//! Managed values injected into node execution context.
//!
//! Stub module for future managed value support (`IsLastStep`,
//! `RemainingSteps`, etc.).

/// Indicates whether the current superstep is the last before the recursion
/// limit is reached.
#[derive(Debug, Clone, Copy)]
pub struct IsLastStep(pub bool);

/// The number of remaining supersteps before the recursion limit.
#[derive(Debug, Clone, Copy)]
pub struct RemainingSteps(pub usize);

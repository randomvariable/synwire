//! Graph construction and execution.
//!
//! This module provides [`StateGraph`] for building typed state machines and
//! [`CompiledGraph`] for executing them. [`ValueState`] provides backward
//! compatibility for existing `serde_json::Value`-based code.

pub mod compiled;
pub mod state;
pub mod value_state;

pub use compiled::CompiledGraph;
pub use state::{ConditionFn, NodeFn, State, StateGraph};
pub use value_state::ValueState;

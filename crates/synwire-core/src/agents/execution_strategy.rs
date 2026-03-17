//! Execution strategies for agents.

use crate::BoxFuture;
use crate::agents::signal::SignalRoute;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Execution strategy error.
#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum StrategyError {
    /// Invalid state transition.
    #[error(
        "Invalid transition from {current_state} via {attempted_action}. Valid actions: {valid_actions:?}"
    )]
    InvalidTransition {
        /// Current state.
        current_state: String,
        /// Attempted action.
        attempted_action: String,
        /// Valid actions from current state.
        valid_actions: Vec<String>,
    },

    /// Guard condition rejected transition.
    #[error("Guard rejected transition: {0}")]
    GuardRejected(String),

    /// No initial state defined.
    #[error("No initial state defined")]
    NoInitialState,

    /// Execution failed.
    #[error("Execution failed: {0}")]
    Execution(String),
}

/// FSM state identifier (newtype for type safety).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FsmStateId(pub String);

impl From<&str> for FsmStateId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for FsmStateId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Action identifier (newtype for type safety).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

impl From<&str> for ActionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ActionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Strategy snapshot for serialization.
pub trait StrategySnapshot: Send + Sync {
    /// Serialize snapshot to JSON.
    fn to_value(&self) -> Result<Value, StrategyError>;
}

/// Execution strategy trait.
///
/// Controls how agent orchestrates actions (immediate vs state-constrained).
pub trait ExecutionStrategy: Send + Sync {
    /// Execute an action with input.
    fn execute<'a>(
        &'a self,
        action: &'a str,
        input: Value,
    ) -> BoxFuture<'a, Result<Value, StrategyError>>;

    /// Process pending work (for stateful strategies).
    fn tick(&self) -> BoxFuture<'_, Result<Option<Value>, StrategyError>>;

    /// Capture current strategy state.
    fn snapshot(&self) -> Result<Box<dyn StrategySnapshot>, StrategyError>;

    /// Get signal routes contributed by this strategy.
    fn signal_routes(&self) -> Vec<SignalRoute> {
        Vec::new()
    }
}

/// Guard condition for FSM transitions.
pub trait GuardCondition: Send + Sync {
    /// Evaluate guard condition.
    fn evaluate(&self, input: &Value) -> bool;

    /// Guard name for error messages.
    fn name(&self) -> &str;
}

/// Closure-based adapter for [`GuardCondition`].
///
/// Wraps an `Fn(&Value) -> bool` closure so it can be used as a guard.
pub struct ClosureGuard {
    name: String,
    f: Box<dyn Fn(&Value) -> bool + Send + Sync>,
}

impl ClosureGuard {
    /// Create a new closure guard with the given name and predicate.
    pub fn new(
        name: impl Into<String>,
        f: impl Fn(&Value) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            f: Box::new(f),
        }
    }
}

impl std::fmt::Debug for ClosureGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosureGuard")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl GuardCondition for ClosureGuard {
    fn evaluate(&self, input: &Value) -> bool {
        (self.f)(input)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

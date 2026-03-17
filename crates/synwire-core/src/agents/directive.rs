//! Directive system for agent effects.

use crate::State;
use crate::agents::streaming::AgentEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Directive - typed effect description returned by agent nodes.
///
/// Directives describe side effects without executing them, enabling pure unit testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum Directive {
    /// Emit an event to the event stream.
    #[serde(rename = "emit")]
    Emit {
        /// Event to emit.
        event: AgentEvent,
    },

    /// Request spawning a child agent.
    #[serde(rename = "spawn_agent")]
    SpawnAgent {
        /// Agent name.
        name: String,
        /// Agent configuration.
        config: Value,
    },

    /// Request stopping a child agent.
    #[serde(rename = "stop_child")]
    StopChild {
        /// Child agent name.
        name: String,
    },

    /// Schedule a delayed action.
    #[serde(rename = "schedule")]
    Schedule {
        /// Action to schedule.
        action: String,
        /// Delay duration.
        #[serde(with = "humantime_serde")]
        delay: Duration,
    },

    /// Request runtime to execute instruction and route result back.
    #[serde(rename = "run_instruction")]
    RunInstruction {
        /// Instruction text.
        instruction: String,
        /// Input data.
        input: Value,
    },

    /// Schedule recurring action.
    #[serde(rename = "cron")]
    Cron {
        /// Cron expression.
        expression: String,
        /// Action to execute.
        action: String,
    },

    /// Request agent stop.
    #[serde(rename = "stop")]
    Stop {
        /// Optional reason.
        reason: Option<String>,
    },

    /// Spawn a background task.
    #[serde(rename = "spawn_task")]
    SpawnTask {
        /// Task description.
        description: String,
        /// Task input data.
        input: Value,
    },

    /// Cancel a background task.
    #[serde(rename = "stop_task")]
    StopTask {
        /// Task ID to stop.
        task_id: String,
    },

    /// User-defined directive (requires typetag registration).
    #[serde(rename = "custom")]
    Custom {
        /// Custom directive payload.
        #[serde(flatten)]
        payload: Box<dyn DirectivePayload>,
    },
}

/// Trait for custom directive payloads.
///
/// Implement this trait and use `#[typetag::serde]` for serialization support.
#[typetag::serde(tag = "custom_type")]
pub trait DirectivePayload: std::fmt::Debug + Send + Sync + dyn_clone::DynClone {}

dyn_clone::clone_trait_object!(DirectivePayload);

/// Result combining state update and zero or more directives.
///
/// Agent nodes return this to indicate both immediate state changes
/// and deferred effects to be executed by the runtime.
#[derive(Debug, Clone)]
pub struct DirectiveResult<S: State> {
    /// Updated state (applied immediately).
    pub state: S,
    /// Deferred effect descriptions (executed by runtime).
    pub directives: Vec<Directive>,
}

impl<S: State> DirectiveResult<S> {
    /// Create a result with only state, no directives.
    #[must_use]
    pub const fn state_only(state: S) -> Self {
        Self {
            state,
            directives: Vec::new(),
        }
    }

    /// Create a result with state and a single directive.
    #[must_use]
    pub fn with_directive(state: S, directive: Directive) -> Self {
        Self {
            state,
            directives: vec![directive],
        }
    }

    /// Create a result with state and multiple directives.
    #[must_use]
    pub const fn with_directives(state: S, directives: Vec<Directive>) -> Self {
        Self { state, directives }
    }
}

impl<S: State> From<S> for DirectiveResult<S> {
    fn from(state: S) -> Self {
        Self::state_only(state)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::redundant_clone
)]
mod tests {
    use super::*;
    use crate::agents::streaming::{AgentEvent, TerminationReason};

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct TestState {
        count: u32,
    }

    impl State for TestState {
        fn to_value(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            Ok(serde_json::to_value(self)?)
        }

        fn from_value(value: Value) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            Ok(serde_json::from_value(value)?)
        }
    }

    #[test]
    fn test_directive_result_state_only() {
        let state = TestState { count: 1 };
        let result = DirectiveResult::state_only(state.clone());
        assert_eq!(result.state, state);
        assert!(result.directives.is_empty());
    }

    #[test]
    fn test_directive_result_with_directive() {
        let state = TestState { count: 1 };
        let directive = Directive::Stop { reason: None };
        let result = DirectiveResult::with_directive(state.clone(), directive.clone());
        assert_eq!(result.state, state);
        assert_eq!(result.directives.len(), 1);
    }

    #[test]
    fn test_directive_result_from_state() {
        let state = TestState { count: 1 };
        let result: DirectiveResult<TestState> = state.clone().into();
        assert_eq!(result.state, state);
        assert!(result.directives.is_empty());
    }

    #[test]
    fn test_directive_serde_emit() {
        let directive = Directive::Emit {
            event: AgentEvent::TurnComplete {
                reason: TerminationReason::Complete,
            },
        };
        let json = serde_json::to_string(&directive).expect("serialize");
        let deserialized: Directive = serde_json::from_str(&json).expect("deserialize");
        // Manual check since AgentEvent doesn't derive PartialEq
        assert!(matches!(deserialized, Directive::Emit { .. }));
    }

    #[test]
    fn test_directive_serde_spawn_agent() {
        let directive = Directive::SpawnAgent {
            name: "helper".to_string(),
            config: serde_json::json!({"model": "gpt-4"}),
        };
        let json = serde_json::to_string(&directive).expect("serialize");
        let deserialized: Directive = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, Directive::SpawnAgent { .. }));
    }

    #[test]
    fn test_directive_serde_stop() {
        let directive = Directive::Stop {
            reason: Some("Task complete".to_string()),
        };
        let json = serde_json::to_string(&directive).expect("serialize");
        let deserialized: Directive = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, Directive::Stop { .. }));
    }
}

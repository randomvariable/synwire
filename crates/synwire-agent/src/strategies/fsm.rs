//! FSM-based execution strategy.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use synwire_core::BoxFuture;
use synwire_core::agents::execution_strategy::{
    ActionId, ExecutionStrategy, FsmStateId, GuardCondition, StrategyError, StrategySnapshot,
};

/// A single FSM transition.
pub struct FsmTransition {
    /// Target state after transition.
    pub target: FsmStateId,
    /// Optional guard that must pass for this transition.
    pub guard: Option<Box<dyn GuardCondition>>,
    /// Priority: higher value evaluated first when multiple transitions share the same key.
    pub priority: i32,
}

impl std::fmt::Debug for FsmTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FsmTransition")
            .field("target", &self.target)
            .field("guard", &self.guard.as_ref().map_or("<none>", |g| g.name()))
            .field("priority", &self.priority)
            .finish()
    }
}

/// Snapshot of FSM strategy state.
#[derive(Debug)]
struct FsmStrategySnapshot {
    current: FsmStateId,
}

impl StrategySnapshot for FsmStrategySnapshot {
    fn to_value(&self) -> Result<Value, StrategyError> {
        Ok(serde_json::json!({
            "type": "fsm",
            "current_state": self.current.0,
        }))
    }
}

/// FSM execution strategy.
///
/// Actions are constrained by a transition table. Only valid actions from the
/// current state are accepted. Guard conditions further restrict transitions.
pub struct FsmStrategy {
    current: Mutex<FsmStateId>,
    /// `(from_state, action) → sorted transitions` where sorting is by descending priority.
    transitions: HashMap<(FsmStateId, ActionId), Vec<FsmTransition>>,
}

impl std::fmt::Debug for FsmStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let current = self
            .current
            .lock()
            .map_or_else(|_| "<poisoned>".to_string(), |g| g.0.clone());
        f.debug_struct("FsmStrategy")
            .field("current_state", &current)
            .finish_non_exhaustive()
    }
}

impl FsmStrategy {
    /// Return a builder for constructing an `FsmStrategy`.
    pub fn builder() -> FsmStrategyBuilder {
        FsmStrategyBuilder::default()
    }

    /// Return the current state ID.
    ///
    /// # Errors
    ///
    /// Returns `StrategyError::Execution` if the internal mutex is poisoned.
    pub fn current_state(&self) -> Result<FsmStateId, StrategyError> {
        self.current
            .lock()
            .map(|g| g.clone())
            .map_err(|_| StrategyError::Execution("mutex poisoned".into()))
    }

    /// Collect valid action IDs from the current state.
    fn valid_actions(&self, from: &FsmStateId) -> Vec<String> {
        self.transitions
            .keys()
            .filter(|(state, _)| state == from)
            .map(|(_, action)| action.0.clone())
            .collect()
    }
}

impl ExecutionStrategy for FsmStrategy {
    fn execute<'a>(
        &'a self,
        action: &'a str,
        input: Value,
    ) -> BoxFuture<'a, Result<Value, StrategyError>> {
        Box::pin(async move {
            let current = self
                .current
                .lock()
                .map_err(|_| StrategyError::Execution("mutex poisoned".into()))?
                .clone();

            let action_id = ActionId::from(action);
            let key = (current.clone(), action_id);

            let transitions =
                self.transitions
                    .get(&key)
                    .ok_or_else(|| StrategyError::InvalidTransition {
                        current_state: current.0.clone(),
                        attempted_action: action.to_string(),
                        valid_actions: self.valid_actions(&current),
                    })?;

            // Find first transition whose guard passes (sorted by descending priority).
            let target = transitions
                .iter()
                .find(|t| t.guard.as_ref().is_none_or(|g| g.evaluate(&input)))
                .map(|t| t.target.clone())
                .ok_or_else(|| {
                    StrategyError::GuardRejected(format!(
                        "all guards rejected transition from '{}' via '{}'",
                        current.0, action
                    ))
                })?;

            *self
                .current
                .lock()
                .map_err(|_| StrategyError::Execution("mutex poisoned".into()))? = target;

            Ok(input)
        })
    }

    fn tick(&self) -> BoxFuture<'_, Result<Option<Value>, StrategyError>> {
        Box::pin(async { Ok(None) })
    }

    fn snapshot(&self) -> Result<Box<dyn StrategySnapshot>, StrategyError> {
        let current = self
            .current
            .lock()
            .map_err(|_| StrategyError::Execution("mutex poisoned".into()))?
            .clone();
        Ok(Box::new(FsmStrategySnapshot { current }))
    }
}

/// Builder for [`FsmStrategy`].
#[derive(Default)]
pub struct FsmStrategyBuilder {
    initial: Option<FsmStateId>,
    transitions: HashMap<(FsmStateId, ActionId), Vec<FsmTransition>>,
    signal_routes: Vec<synwire_core::agents::signal::SignalRoute>,
}

impl FsmStrategyBuilder {
    /// Declare an FSM state (currently just for documentation).
    #[must_use]
    pub fn state(self, _state: impl Into<FsmStateId>) -> Self {
        // States are inferred from transitions; this method exists for
        // readability and future validation.
        self
    }

    /// Add a transition without a guard.
    #[must_use]
    pub fn transition(
        mut self,
        from: impl Into<FsmStateId>,
        action: impl Into<ActionId>,
        to: impl Into<FsmStateId>,
    ) -> Self {
        let key = (from.into(), action.into());
        self.transitions
            .entry(key)
            .or_default()
            .push(FsmTransition {
                target: to.into(),
                guard: None,
                priority: 0,
            });
        self
    }

    /// Add a guarded transition.
    #[must_use]
    pub fn transition_with_guard(
        mut self,
        from: impl Into<FsmStateId>,
        action: impl Into<ActionId>,
        to: impl Into<FsmStateId>,
        guard: impl GuardCondition + 'static,
        priority: i32,
    ) -> Self {
        let key = (from.into(), action.into());
        self.transitions
            .entry(key)
            .or_default()
            .push(FsmTransition {
                target: to.into(),
                guard: Some(Box::new(guard)),
                priority,
            });
        self
    }

    /// Add a signal route contributed by this strategy.
    #[must_use]
    pub fn route(mut self, route: synwire_core::agents::signal::SignalRoute) -> Self {
        self.signal_routes.push(route);
        self
    }

    /// Set the initial state.
    #[must_use]
    pub fn initial(mut self, state: impl Into<FsmStateId>) -> Self {
        self.initial = Some(state.into());
        self
    }

    /// Build the `FsmStrategy`.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyError::NoInitialState`] if no initial state was set.
    pub fn build(mut self) -> Result<FsmStrategyWithRoutes, StrategyError> {
        let initial = self.initial.ok_or(StrategyError::NoInitialState)?;

        // Sort transitions by descending priority.
        for transitions in self.transitions.values_mut() {
            transitions.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        Ok(FsmStrategyWithRoutes {
            strategy: FsmStrategy {
                current: Mutex::new(initial),
                transitions: self.transitions,
            },
            signal_routes: self.signal_routes,
        })
    }
}

/// `FsmStrategy` bundled with its signal routes.
pub struct FsmStrategyWithRoutes {
    /// The FSM strategy.
    pub strategy: FsmStrategy,
    signal_routes: Vec<synwire_core::agents::signal::SignalRoute>,
}

impl std::fmt::Debug for FsmStrategyWithRoutes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FsmStrategyWithRoutes")
            .field("strategy", &self.strategy)
            .finish_non_exhaustive()
    }
}

impl ExecutionStrategy for FsmStrategyWithRoutes {
    fn execute<'a>(
        &'a self,
        action: &'a str,
        input: Value,
    ) -> BoxFuture<'a, Result<Value, StrategyError>> {
        self.strategy.execute(action, input)
    }

    fn tick(&self) -> BoxFuture<'_, Result<Option<Value>, StrategyError>> {
        self.strategy.tick()
    }

    fn snapshot(&self) -> Result<Box<dyn StrategySnapshot>, StrategyError> {
        self.strategy.snapshot()
    }

    fn signal_routes(&self) -> Vec<synwire_core::agents::signal::SignalRoute> {
        self.signal_routes.clone()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use synwire_core::agents::execution_strategy::ClosureGuard;

    fn build_simple_fsm() -> FsmStrategyWithRoutes {
        FsmStrategy::builder()
            .initial("idle")
            .state("idle")
            .state("running")
            .state("done")
            .transition("idle", "start", "running")
            .transition("running", "finish", "done")
            .build()
            .expect("valid FSM")
    }

    #[tokio::test]
    async fn test_valid_transition() {
        let fsm = build_simple_fsm();
        let _ = fsm
            .execute("start", serde_json::json!({}))
            .await
            .expect("transition");
        assert_eq!(fsm.strategy.current_state().expect("state").0, "running");
    }

    #[tokio::test]
    async fn test_invalid_transition_returns_error() {
        let fsm = build_simple_fsm();
        let err = fsm
            .execute("finish", serde_json::json!({}))
            .await
            .expect_err("should fail");
        assert!(matches!(err, StrategyError::InvalidTransition { .. }));
        // Error message includes current state and attempted action.
        let msg = err.to_string();
        assert!(msg.contains("idle"));
        assert!(msg.contains("finish"));
    }

    #[tokio::test]
    async fn test_guard_rejection() {
        let guard = ClosureGuard::new("never", |_| false);
        let fsm = FsmStrategy::builder()
            .initial("idle")
            .transition_with_guard("idle", "start", "running", guard, 0)
            .build()
            .expect("valid FSM");

        let err = fsm
            .execute("start", serde_json::json!({}))
            .await
            .expect_err("guard should reject");
        assert!(matches!(err, StrategyError::GuardRejected(_)));
    }

    #[tokio::test]
    async fn test_guard_priority_order() {
        // Two guards for the same transition; higher priority runs first.
        let allow_guard = ClosureGuard::new("allow", |_| true);
        let deny_guard = ClosureGuard::new("deny", |_| false);
        let fsm = FsmStrategy::builder()
            .initial("idle")
            // deny has higher priority but allow also matches – first that passes wins.
            .transition_with_guard("idle", "start", "deny-target", deny_guard, 10)
            .transition_with_guard("idle", "start", "allow-target", allow_guard, 5)
            .build()
            .expect("valid FSM");

        // deny guard fires first (priority 10) but fails; allow guard (priority 5) passes.
        let _ = fsm
            .execute("start", serde_json::json!({}))
            .await
            .expect("allow guard should pass");
        assert_eq!(
            fsm.strategy.current_state().expect("state").0,
            "allow-target"
        );
    }

    #[tokio::test]
    async fn test_snapshot_serialization() {
        let fsm = build_simple_fsm();
        let snap = fsm.snapshot().expect("snapshot");
        let val = snap.to_value().expect("serialize");
        assert_eq!(val["type"], "fsm");
        assert_eq!(val["current_state"], "idle");
    }

    #[test]
    fn test_builder_requires_initial_state() {
        let err = FsmStrategy::builder()
            .transition("a", "go", "b")
            .build()
            .expect_err("should need initial state");
        assert!(matches!(err, StrategyError::NoInitialState));
    }
}

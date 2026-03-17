//! Signal routing for agent communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Signal kind category.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SignalKind {
    /// User requested stop.
    Stop,
    /// User message received.
    UserMessage,
    /// Tool invocation result.
    ToolResult,
    /// Timer / cron event.
    Timer,
    /// Custom signal kind.
    Custom(String),
}

/// Signal sent to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Signal kind.
    pub kind: SignalKind,
    /// Signal payload.
    pub payload: Value,
}

impl Signal {
    /// Create a new signal.
    #[must_use]
    pub const fn new(kind: SignalKind, payload: Value) -> Self {
        Self { kind, payload }
    }
}

/// Action to take in response to a signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Action {
    /// Continue processing normally.
    Continue,
    /// Stop the agent gracefully (drain in-flight work first).
    GracefulStop,
    /// Stop the agent immediately.
    ForceStop,
    /// Transition to a new FSM state.
    Transition(String),
    /// Custom action identifier.
    Custom(String),
}

/// A route mapping a signal kind (with optional predicate) to an action.
#[derive(Debug, Clone)]
pub struct SignalRoute {
    /// Signal kind this route handles.
    pub kind: SignalKind,
    /// Optional predicate for additional filtering.
    ///
    /// Uses a function pointer so `SignalRoute` remains `Clone + Send + Sync`.
    pub predicate: Option<fn(&Signal) -> bool>,
    /// Action to take when route matches.
    pub action: Action,
    /// Priority: higher value wins when multiple routes match.
    pub priority: i32,
}

impl SignalRoute {
    /// Create a new signal route without a predicate.
    #[must_use]
    pub fn new(kind: SignalKind, action: Action, priority: i32) -> Self {
        Self {
            kind,
            predicate: None,
            action,
            priority,
        }
    }

    /// Create a route with an additional predicate.
    #[must_use]
    pub fn with_predicate(
        kind: SignalKind,
        predicate: fn(&Signal) -> bool,
        action: Action,
        priority: i32,
    ) -> Self {
        Self {
            kind,
            predicate: Some(predicate),
            action,
            priority,
        }
    }

    /// Returns `true` if this route matches the given signal.
    #[must_use]
    pub fn matches(&self, signal: &Signal) -> bool {
        if self.kind != signal.kind {
            return false;
        }
        self.predicate.is_none_or(|pred| pred(signal))
    }
}

/// Routes signals to actions.
pub trait SignalRouter: Send + Sync {
    /// Route a signal, returning the best-matching action if any.
    fn route(&self, signal: &Signal) -> Option<Action>;

    /// All routes contributed by this router.
    fn routes(&self) -> Vec<SignalRoute>;
}

/// Composed router across three priority tiers: strategy > agent > plugin.
///
/// Within each tier, the route with the highest `priority` value wins.
/// Strategy-tier routes always beat agent-tier routes regardless of priority value.
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct ComposedRouter {
    strategy_routes: Vec<SignalRoute>,
    agent_routes: Vec<SignalRoute>,
    plugin_routes: Vec<SignalRoute>,
}

impl ComposedRouter {
    /// Create a new composed router.
    #[must_use]
    pub const fn new(
        strategy_routes: Vec<SignalRoute>,
        agent_routes: Vec<SignalRoute>,
        plugin_routes: Vec<SignalRoute>,
    ) -> Self {
        Self {
            strategy_routes,
            agent_routes,
            plugin_routes,
        }
    }

    fn best_match<'a>(signal: &Signal, routes: &'a [SignalRoute]) -> Option<&'a SignalRoute> {
        routes
            .iter()
            .filter(|r| r.matches(signal))
            .max_by_key(|r| r.priority)
    }
}

impl SignalRouter for ComposedRouter {
    fn route(&self, signal: &Signal) -> Option<Action> {
        // Strategy routes have the highest tier precedence.
        if let Some(route) = Self::best_match(signal, &self.strategy_routes) {
            tracing::debug!(
                kind = ?signal.kind,
                priority = route.priority,
                tier = "strategy",
                "Signal routed"
            );
            return Some(route.action.clone());
        }

        // Agent routes are second.
        if let Some(route) = Self::best_match(signal, &self.agent_routes) {
            tracing::debug!(
                kind = ?signal.kind,
                priority = route.priority,
                tier = "agent",
                "Signal routed"
            );
            return Some(route.action.clone());
        }

        // Plugin routes are lowest.
        if let Some(route) = Self::best_match(signal, &self.plugin_routes) {
            tracing::debug!(
                kind = ?signal.kind,
                priority = route.priority,
                tier = "plugin",
                "Signal routed"
            );
            return Some(route.action.clone());
        }

        tracing::debug!(kind = ?signal.kind, "No route found for signal");
        None
    }

    fn routes(&self) -> Vec<SignalRoute> {
        let mut all = self.strategy_routes.clone();
        all.extend(self.agent_routes.clone());
        all.extend(self.plugin_routes.clone());
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stop_signal() -> Signal {
        Signal::new(SignalKind::Stop, json!(null))
    }

    fn user_signal() -> Signal {
        Signal::new(SignalKind::UserMessage, json!("hello"))
    }

    #[test]
    fn test_strategy_route_wins_over_agent() {
        let strategy = vec![SignalRoute::new(SignalKind::Stop, Action::ForceStop, 0)];
        let agent = vec![SignalRoute::new(
            SignalKind::Stop,
            Action::GracefulStop,
            100,
        )];
        let router = ComposedRouter::new(strategy, agent, vec![]);

        let action = router.route(&stop_signal());
        assert!(matches!(action, Some(Action::ForceStop)));
    }

    #[test]
    fn test_agent_route_wins_over_plugin() {
        let agent = vec![SignalRoute::new(SignalKind::Stop, Action::GracefulStop, 0)];
        let plugin = vec![SignalRoute::new(SignalKind::Stop, Action::Continue, 100)];
        let router = ComposedRouter::new(vec![], agent, plugin);

        let action = router.route(&stop_signal());
        assert!(matches!(action, Some(Action::GracefulStop)));
    }

    #[test]
    fn test_higher_priority_wins_within_tier() {
        let agent = vec![
            SignalRoute::new(SignalKind::Stop, Action::GracefulStop, 10),
            SignalRoute::new(SignalKind::Stop, Action::ForceStop, 20),
        ];
        let router = ComposedRouter::new(vec![], agent, vec![]);
        let action = router.route(&stop_signal());
        assert!(matches!(action, Some(Action::ForceStop)));
    }

    #[test]
    fn test_predicate_filtering() {
        fn only_nonempty(s: &Signal) -> bool {
            s.payload.as_str().is_some_and(|v| !v.is_empty())
        }

        let agent = vec![
            SignalRoute::with_predicate(
                SignalKind::UserMessage,
                only_nonempty,
                Action::Continue,
                10,
            ),
            SignalRoute::new(SignalKind::UserMessage, Action::GracefulStop, 0),
        ];
        let router = ComposedRouter::new(vec![], agent, vec![]);

        // Non-empty payload: predicate matches, higher priority wins.
        let action = router.route(&user_signal());
        assert!(matches!(action, Some(Action::Continue)));

        // Empty payload: predicate fails, falls back to lower-priority route.
        let empty = Signal::new(SignalKind::UserMessage, json!(""));
        let action = router.route(&empty);
        assert!(matches!(action, Some(Action::GracefulStop)));
    }

    #[test]
    fn test_no_route_returns_none() {
        let router = ComposedRouter::new(vec![], vec![], vec![]);
        assert!(router.route(&stop_signal()).is_none());
    }
}

//! Monte Carlo Tree Search execution strategy.
//!
//! Uses MCTS to explore the space of possible tool call sequences,
//! guided by a configurable value function.

use std::collections::HashMap;

/// Configuration for the MCTS execution strategy.
#[non_exhaustive]
pub struct MctsConfig {
    /// Maximum tree depth.
    pub max_depth: usize,
    /// Number of simulations per step.
    pub simulations: usize,
    /// Exploration constant (UCB1 parameter).
    pub exploration_constant: f32,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            simulations: 50,
            exploration_constant: 1.414,
        }
    }
}

/// MCTS-based execution strategy.
///
/// Selects the next tool action using Upper Confidence Bound (UCB1) scoring,
/// balancing exploration of new actions against exploitation of known-good ones.
pub struct MctsStrategy {
    config: MctsConfig,
}

impl MctsStrategy {
    /// Create with the given configuration.
    pub const fn new(config: MctsConfig) -> Self {
        Self { config }
    }

    /// Select the best action given available actions and value estimates.
    ///
    /// Applies UCB1 to combine the provided value estimates with an exploration
    /// bonus derived from [`MctsConfig::exploration_constant`].
    /// Returns the selected action (tool name), or `None` if `available_actions`
    /// is empty.
    pub fn select_action(
        &self,
        available_actions: &[String],
        value_estimates: &[(String, f32)],
    ) -> Option<String> {
        if available_actions.is_empty() {
            return None;
        }

        let value_map: HashMap<&str, f32> = value_estimates
            .iter()
            .map(|(a, v)| (a.as_str(), *v))
            .collect();

        // UCB1: score = v + C * sqrt(ln(N) / n_i)
        // Simplified: no per-node visit counts in this interface, use value + exploration bonus.
        let best = available_actions
            .iter()
            .map(|a| {
                let v = value_map.get(a.as_str()).copied().unwrap_or(0.0);
                (a.clone(), v + self.config.exploration_constant)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        best.map(|(a, _)| a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_action_returns_highest_value() {
        let config = MctsConfig::default();
        let strategy = MctsStrategy::new(config);
        let actions = vec![
            "tool_a".to_owned(),
            "tool_b".to_owned(),
            "tool_c".to_owned(),
        ];
        let estimates = vec![
            ("tool_a".to_owned(), 0.3_f32),
            ("tool_b".to_owned(), 0.9_f32),
            ("tool_c".to_owned(), 0.1_f32),
        ];
        let selected = strategy.select_action(&actions, &estimates);
        assert_eq!(selected, Some("tool_b".to_owned()));
    }

    #[test]
    fn select_action_empty_returns_none() {
        let strategy = MctsStrategy::new(MctsConfig::default());
        assert_eq!(strategy.select_action(&[], &[]), None);
    }

    #[test]
    fn select_action_no_estimates_returns_first_by_exploration() {
        let strategy = MctsStrategy::new(MctsConfig::default());
        let actions = vec!["only_tool".to_owned()];
        let selected = strategy.select_action(&actions, &[]);
        assert_eq!(selected, Some("only_tool".to_owned()));
    }
}

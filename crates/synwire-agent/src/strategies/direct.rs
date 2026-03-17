//! Direct execution strategy - executes actions immediately.

use serde_json::Value;
use synwire_core::BoxFuture;
use synwire_core::agents::execution_strategy::{
    ExecutionStrategy, StrategyError, StrategySnapshot,
};

/// Direct execution strategy - executes actions immediately without state checks.
#[derive(Debug, Default, Clone)]
pub struct DirectStrategy;

impl DirectStrategy {
    /// Create a new direct strategy.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

struct DirectSnapshot;

impl StrategySnapshot for DirectSnapshot {
    fn to_value(&self) -> Result<Value, StrategyError> {
        Ok(serde_json::json!({"type": "direct"}))
    }
}

impl ExecutionStrategy for DirectStrategy {
    fn execute<'a>(
        &'a self,
        _action: &'a str,
        input: Value,
    ) -> BoxFuture<'a, Result<Value, StrategyError>> {
        // Direct strategy: just pass through the input as output
        Box::pin(async move { Ok(input) })
    }

    fn tick(&self) -> BoxFuture<'_, Result<Option<Value>, StrategyError>> {
        // No pending work in direct strategy
        Box::pin(async { Ok(None) })
    }

    fn snapshot(&self) -> Result<Box<dyn StrategySnapshot>, StrategyError> {
        Ok(Box::new(DirectSnapshot))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_strategy_passthrough() {
        let strategy = DirectStrategy::new();
        let input = serde_json::json!({"test": "data"});
        let result = strategy
            .execute("any_action", input.clone())
            .await
            .expect("execute");
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_direct_strategy_no_pending_work() {
        let strategy = DirectStrategy::new();
        let result = strategy.tick().await.expect("tick");
        assert!(result.is_none());
    }
}

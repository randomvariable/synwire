//! Directive execution traits and implementations.

use crate::BoxFuture;
use crate::agents::directive::Directive;
use serde_json::Value;
use thiserror::Error;

/// Directive execution error.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DirectiveError {
    /// Execution failed.
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Unsupported directive type.
    #[error("Unsupported directive: {0}")]
    Unsupported(String),
}

/// Executes directives and produces optional results.
///
/// The default implementation is `NoOpExecutor` which records without executing.
pub trait DirectiveExecutor: Send + Sync {
    /// Execute a directive, returning an optional result value.
    ///
    /// Returns `Some(value)` for directives like `RunInstruction` that need a result
    /// routed back to the agent.
    fn execute_directive(
        &self,
        directive: &Directive,
    ) -> BoxFuture<'_, Result<Option<Value>, DirectiveError>>;
}

/// No-op executor that records directives without executing them.
///
/// Useful for testing - directives are collected but no side effects occur.
#[derive(Debug, Default, Clone)]
pub struct NoOpExecutor;

impl DirectiveExecutor for NoOpExecutor {
    fn execute_directive(
        &self,
        _directive: &Directive,
    ) -> BoxFuture<'_, Result<Option<Value>, DirectiveError>> {
        Box::pin(async { Ok(None) })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_executor_returns_none() {
        let executor = NoOpExecutor;
        let directive = Directive::Stop { reason: None };
        let result = executor
            .execute_directive(&directive)
            .await
            .expect("execute");
        assert!(result.is_none());
    }
}

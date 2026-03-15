//! Conditional branching runnable.

use std::sync::Arc;

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::runnables::core::RunnableCore;
use serde_json::Value;

/// Type alias for a branch condition function.
type ConditionFn = Arc<dyn Fn(&Value) -> bool + Send + Sync>;

/// Routes input to different runnables based on conditions.
///
/// Evaluates conditions in order; the first branch whose condition
/// returns `true` is invoked. If no conditions match, the default
/// runnable is invoked.
///
/// # Example
///
/// ```rust,no_run
/// # use std::sync::Arc;
/// # use synwire_core::runnables::{RunnableBranch, RunnablePassthrough, RunnableCore};
/// let branch = RunnableBranch::new(
///     vec![
///         (Arc::new(|v: &serde_json::Value| v.is_number()), Box::new(RunnablePassthrough) as Box<dyn RunnableCore>),
///     ],
///     Box::new(RunnablePassthrough),
/// );
/// ```
pub struct RunnableBranch {
    branches: Vec<(ConditionFn, Box<dyn RunnableCore>)>,
    default: Box<dyn RunnableCore>,
}

impl RunnableBranch {
    /// Create a new branch with condition-runnable pairs and a default.
    pub fn new(
        branches: Vec<(ConditionFn, Box<dyn RunnableCore>)>,
        default: Box<dyn RunnableCore>,
    ) -> Self {
        Self { branches, default }
    }
}

impl RunnableCore for RunnableBranch {
    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        Box::pin(async move {
            for (condition, runnable) in &self.branches {
                if condition(&input) {
                    return runnable.invoke(input, config).await;
                }
            }
            self.default.invoke(input, config).await
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "RunnableBranch"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runnables::lambda::RunnableLambda;
    use crate::runnables::passthrough::RunnablePassthrough;

    #[tokio::test]
    async fn test_branch_routes_correctly() {
        let is_number: ConditionFn = Arc::new(|v: &Value| v.is_number());
        let double = RunnableLambda::new(|v: Value| {
            Box::pin(async move {
                let n = v.as_i64().unwrap() * 2;
                Ok(Value::from(n))
            })
        });

        let branch = RunnableBranch::new(
            vec![(is_number, Box::new(double) as Box<dyn RunnableCore>)],
            Box::new(RunnablePassthrough),
        );

        // Number input should be doubled.
        let result = branch.invoke(Value::from(5), None).await.unwrap();
        assert_eq!(result, Value::from(10));

        // String input should pass through via default.
        let result = branch.invoke(Value::from("hello"), None).await.unwrap();
        assert_eq!(result, Value::from("hello"));
    }

    #[tokio::test]
    async fn test_branch_default_when_no_match() {
        let never_true: ConditionFn = Arc::new(|_: &Value| false);
        let branch = RunnableBranch::new(
            vec![(
                never_true,
                Box::new(RunnablePassthrough) as Box<dyn RunnableCore>,
            )],
            Box::new(RunnableLambda::new(|_| {
                Box::pin(async { Ok(Value::from("default")) })
            })),
        );

        let result = branch.invoke(Value::from(1), None).await.unwrap();
        assert_eq!(result, Value::from("default"));
    }
}

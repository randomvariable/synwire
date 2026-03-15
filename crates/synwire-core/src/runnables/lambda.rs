//! Lambda runnable wrapping an arbitrary async closure.

use std::sync::Arc;

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::runnables::core::RunnableCore;
use serde_json::Value;

/// Type alias for the wrapped lambda function.
type LambdaFn = Arc<dyn Fn(Value) -> BoxFuture<'static, Result<Value, SynwireError>> + Send + Sync>;

/// A runnable that wraps an async closure.
///
/// # Example
///
/// ```rust,no_run
/// # use synwire_core::runnables::{RunnableLambda, RunnableCore};
/// # use serde_json::Value;
/// let double = RunnableLambda::new(|v: Value| {
///     Box::pin(async move {
///         let n = v.as_i64().unwrap_or(0) * 2;
///         Ok(Value::from(n))
///     })
/// });
/// ```
pub struct RunnableLambda {
    func: LambdaFn,
    name: String,
}

impl RunnableLambda {
    /// Create a new lambda runnable from an async closure.
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(Value) -> BoxFuture<'static, Result<Value, SynwireError>> + Send + Sync + 'static,
    {
        Self {
            func: Arc::new(func),
            name: "RunnableLambda".into(),
        }
    }

    /// Set a custom name for this lambda.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl RunnableCore for RunnableLambda {
    fn invoke<'a>(
        &'a self,
        input: Value,
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        (self.func)(input)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lambda_invokes() {
        let lambda = RunnableLambda::new(|v: Value| {
            Box::pin(async move {
                let s = v.as_str().unwrap_or("").to_uppercase();
                Ok(Value::from(s))
            })
        });

        let result = lambda.invoke(Value::from("hello"), None).await.unwrap();
        assert_eq!(result, Value::from("HELLO"));
    }

    #[tokio::test]
    async fn test_lambda_with_name() {
        let lambda =
            RunnableLambda::new(|v: Value| Box::pin(async move { Ok(v) })).with_name("my_func");
        assert_eq!(lambda.name(), "my_func");
    }

    #[tokio::test]
    async fn test_lambda_default_name() {
        let lambda = RunnableLambda::new(|v: Value| Box::pin(async move { Ok(v) }));
        assert_eq!(lambda.name(), "RunnableLambda");
    }
}

//! Fallback composition for runnables.

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::runnables::core::RunnableCore;
use serde_json::Value;

/// A runnable that tries a primary and falls back to alternatives on failure.
///
/// Tries the primary runnable first. If it returns an error, each fallback
/// is tried in order until one succeeds. If all fail, the last error is
/// returned.
pub struct RunnableWithFallbacks {
    primary: Box<dyn RunnableCore>,
    fallbacks: Vec<Box<dyn RunnableCore>>,
}

impl RunnableWithFallbacks {
    /// Create a new fallback composition.
    pub fn new(primary: Box<dyn RunnableCore>, fallbacks: Vec<Box<dyn RunnableCore>>) -> Self {
        Self { primary, fallbacks }
    }
}

impl RunnableCore for RunnableWithFallbacks {
    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        Box::pin(async move {
            let mut last_error = match self.primary.invoke(input.clone(), config).await {
                Ok(v) => return Ok(v),
                Err(e) => e,
            };

            for fallback in &self.fallbacks {
                match fallback.invoke(input.clone(), config).await {
                    Ok(v) => return Ok(v),
                    Err(e) => last_error = e,
                }
            }

            Err(last_error)
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "RunnableWithFallbacks"
    }
}

/// Convenience function to compose a primary runnable with fallbacks.
pub fn with_fallbacks(
    primary: Box<dyn RunnableCore>,
    fallbacks: Vec<Box<dyn RunnableCore>>,
) -> RunnableWithFallbacks {
    RunnableWithFallbacks::new(primary, fallbacks)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runnables::lambda::RunnableLambda;

    #[tokio::test]
    async fn test_fallback_on_primary_failure() {
        let failing = RunnableLambda::new(|_: Value| {
            Box::pin(async { Err(SynwireError::Other("primary failed".into())) })
        });

        let fallback =
            RunnableLambda::new(|_: Value| Box::pin(async { Ok(Value::from("fallback_result")) }));

        let composed = with_fallbacks(Box::new(failing), vec![Box::new(fallback)]);
        let result = composed.invoke(Value::from("input"), None).await.unwrap();
        assert_eq!(result, Value::from("fallback_result"));
    }

    #[tokio::test]
    async fn test_primary_succeeds_no_fallback() {
        let primary = RunnableLambda::new(|v: Value| Box::pin(async { Ok(v) }));
        let fallback =
            RunnableLambda::new(|_: Value| Box::pin(async { Ok(Value::from("should_not_reach")) }));

        let composed = with_fallbacks(Box::new(primary), vec![Box::new(fallback)]);
        let result = composed
            .invoke(Value::from("original"), None)
            .await
            .unwrap();
        assert_eq!(result, Value::from("original"));
    }

    #[tokio::test]
    async fn test_all_fallbacks_fail() {
        let failing = RunnableLambda::new(|_: Value| {
            Box::pin(async { Err(SynwireError::Other("fail".into())) })
        });
        let also_failing = RunnableLambda::new(|_: Value| {
            Box::pin(async { Err(SynwireError::Other("also fail".into())) })
        });

        let composed = with_fallbacks(Box::new(failing), vec![Box::new(also_failing)]);
        let result = composed.invoke(Value::from("input"), None).await;
        assert!(result.is_err());
    }
}

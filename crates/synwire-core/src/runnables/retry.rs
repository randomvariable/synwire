//! Retry configuration types and retry-wrapped runnables.

use crate::error::SynwireErrorKind;
use std::time::Duration;

/// Configuration for retry behaviour.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Error kinds to retry on.
    pub retry_on: Vec<SynwireErrorKind>,
    /// Maximum number of attempts.
    pub max_attempts: u32,
    /// Whether to use exponential backoff with jitter.
    pub wait_exponential_jitter: bool,
    /// Initial interval between retries.
    pub initial_interval: Duration,
    /// Maximum interval between retries.
    pub max_interval: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            retry_on: Vec::new(),
            max_attempts: 3,
            wait_exponential_jitter: true,
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(60),
        }
    }
}

/// State tracked during retry attempts.
#[derive(Debug)]
pub struct RetryState {
    /// Current attempt number.
    pub attempt: u32,
    /// The error that triggered the retry.
    pub error: crate::error::SynwireError,
    /// Total elapsed time since first attempt.
    pub elapsed: Duration,
}

// --- RunnableRetry implementation ---

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::runnables::core::RunnableCore;
use serde_json::Value;

/// A runnable that retries an inner runnable on matching errors.
///
/// Uses exponential backoff with optional jitter. Only errors whose
/// [`SynwireErrorKind`] appears in the `retry_on` list are retried;
/// all other errors propagate immediately.
pub struct RunnableRetry {
    inner: Box<dyn RunnableCore>,
    config: RetryConfig,
}

impl RunnableRetry {
    /// Wrap a runnable with retry behaviour.
    pub fn new(inner: Box<dyn RunnableCore>, config: RetryConfig) -> Self {
        Self { inner, config }
    }

    /// Determine whether an error should be retried.
    fn should_retry(&self, err: &SynwireError) -> bool {
        if self.config.retry_on.is_empty() {
            return true;
        }
        self.config.retry_on.contains(&err.kind())
    }

    /// Compute backoff duration for a given attempt (0-indexed).
    fn backoff_duration(&self, attempt: u32) -> Duration {
        let base = self
            .config
            .initial_interval
            .saturating_mul(1u32.checked_shl(attempt).unwrap_or(u32::MAX));
        base.min(self.config.max_interval)
    }
}

impl RunnableCore for RunnableRetry {
    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        Box::pin(async move {
            let mut last_error: Option<SynwireError> = None;

            for attempt in 0..self.config.max_attempts {
                match self.inner.invoke(input.clone(), config).await {
                    Ok(v) => return Ok(v),
                    Err(e) => {
                        if !self.should_retry(&e) || attempt + 1 >= self.config.max_attempts {
                            return Err(e);
                        }
                        let delay = self.backoff_duration(attempt);
                        tokio::time::sleep(delay).await;
                        last_error = Some(e);
                    }
                }
            }

            // This branch is only reachable if max_attempts == 0.
            Err(last_error
                .unwrap_or_else(|| SynwireError::Other("retry exhausted with no attempts".into())))
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "RunnableRetry"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runnables::lambda::RunnableLambda;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_retry_on_error() {
        let call_count = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&call_count);

        let flaky = RunnableLambda::new(move |v: Value| {
            let count = Arc::clone(&count);
            Box::pin(async move {
                let n = count.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(SynwireError::Other("transient".into()))
                } else {
                    Ok(v)
                }
            })
        });

        let retry_config = RetryConfig {
            max_attempts: 5,
            initial_interval: Duration::from_millis(1),
            max_interval: Duration::from_millis(10),
            ..RetryConfig::default()
        };

        let retried = RunnableRetry::new(Box::new(flaky), retry_config);
        let result = retried.invoke(Value::from("ok"), None).await.unwrap();
        assert_eq!(result, Value::from("ok"));
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_respects_max_attempts() {
        let always_fail = RunnableLambda::new(|_: Value| {
            Box::pin(async { Err(SynwireError::Other("always fails".into())) })
        });

        let retry_config = RetryConfig {
            max_attempts: 2,
            initial_interval: Duration::from_millis(1),
            max_interval: Duration::from_millis(1),
            ..RetryConfig::default()
        };

        let retried = RunnableRetry::new(Box::new(always_fail), retry_config);
        let result = retried.invoke(Value::from("input"), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_skips_non_matching_errors() {
        let tool_err = RunnableLambda::new(|_: Value| {
            Box::pin(async {
                Err(SynwireError::Prompt {
                    message: "bad prompt".into(),
                })
            })
        });

        let retry_config = RetryConfig {
            retry_on: vec![SynwireErrorKind::Model], // only retry model errors
            max_attempts: 3,
            initial_interval: Duration::from_millis(1),
            max_interval: Duration::from_millis(1),
            ..RetryConfig::default()
        };

        let retried = RunnableRetry::new(Box::new(tool_err), retry_config);
        let result = retried.invoke(Value::from("input"), None).await;
        assert!(result.is_err());
    }
}

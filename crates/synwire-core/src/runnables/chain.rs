//! Runnable composition: sequences and parallel execution.

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::runnables::core::RunnableCore;
use serde_json::Value;

/// A sequence of runnables executed one after another.
///
/// The output of each step becomes the input to the next.
///
/// # Example
///
/// ```rust,no_run
/// # use synwire_core::runnables::{RunnableSequence, RunnablePassthrough, RunnableCore};
/// let seq = RunnableSequence::new(vec![
///     Box::new(RunnablePassthrough),
///     Box::new(RunnablePassthrough),
/// ]);
/// ```
pub struct RunnableSequence {
    steps: Vec<Box<dyn RunnableCore>>,
    name: Option<String>,
}

impl RunnableSequence {
    /// Create a new sequence from an ordered list of steps.
    pub fn new(steps: Vec<Box<dyn RunnableCore>>) -> Self {
        Self { steps, name: None }
    }

    /// Set a custom name for this sequence.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl RunnableCore for RunnableSequence {
    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        Box::pin(async move {
            let mut current = input;
            for step in &self.steps {
                current = step.invoke(current, config).await?;
            }
            Ok(current)
        })
    }

    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("RunnableSequence")
    }
}

/// Compose two runnables into a sequence.
///
/// The output of `first` is fed as input to `second`.
pub fn pipe(first: Box<dyn RunnableCore>, second: Box<dyn RunnableCore>) -> RunnableSequence {
    RunnableSequence::new(vec![first, second])
}

/// Executes named runnables concurrently and collects results as a JSON object.
///
/// Each step receives a clone of the input. Results are collected into a
/// `serde_json::Value::Object` keyed by the step names.
///
/// # Example
///
/// ```rust,no_run
/// # use synwire_core::runnables::{RunnableParallel, RunnablePassthrough, RunnableCore};
/// let par = RunnableParallel::new(vec![
///     ("a".into(), Box::new(RunnablePassthrough) as Box<dyn RunnableCore>),
///     ("b".into(), Box::new(RunnablePassthrough) as Box<dyn RunnableCore>),
/// ]);
/// ```
pub struct RunnableParallel {
    steps: Vec<(String, Box<dyn RunnableCore>)>,
}

impl RunnableParallel {
    /// Create from named steps.
    pub fn new(steps: Vec<(String, Box<dyn RunnableCore>)>) -> Self {
        Self { steps }
    }
}

impl RunnableCore for RunnableParallel {
    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        Box::pin(async move {
            let futures: Vec<_> = self
                .steps
                .iter()
                .map(|(name, runnable)| {
                    let input_clone = input.clone();
                    let name = name.clone();
                    async move {
                        let result = runnable.invoke(input_clone, config).await?;
                        Ok::<_, SynwireError>((name, result))
                    }
                })
                .collect();

            let results = futures_util::future::try_join_all(futures).await?;
            let mut map = serde_json::Map::new();
            for (name, value) in results {
                let _replaced = map.insert(name, value);
            }
            Ok(Value::Object(map))
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "RunnableParallel"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runnables::lambda::RunnableLambda;
    use crate::runnables::passthrough::RunnablePassthrough;

    #[tokio::test]
    async fn test_runnable_sequence() {
        let add_one = RunnableLambda::new(|v: Value| {
            Box::pin(async move {
                let n = v.as_i64().unwrap() + 1;
                Ok(Value::from(n))
            })
        });
        let multiply_two = RunnableLambda::new(|v: Value| {
            Box::pin(async move {
                let n = v.as_i64().unwrap() * 2;
                Ok(Value::from(n))
            })
        });

        let seq = RunnableSequence::new(vec![Box::new(add_one), Box::new(multiply_two)]);
        let result = seq.invoke(Value::from(5), None).await.unwrap();
        assert_eq!(result, Value::from(12)); // (5 + 1) * 2
    }

    #[tokio::test]
    async fn test_pipe_composes() {
        let add_one = RunnableLambda::new(|v: Value| {
            Box::pin(async move {
                let n = v.as_i64().unwrap() + 1;
                Ok(Value::from(n))
            })
        });
        let multiply_two = RunnableLambda::new(|v: Value| {
            Box::pin(async move {
                let n = v.as_i64().unwrap() * 2;
                Ok(Value::from(n))
            })
        });

        let seq = pipe(Box::new(add_one), Box::new(multiply_two));
        let result = seq.invoke(Value::from(10), None).await.unwrap();
        assert_eq!(result, Value::from(22)); // (10 + 1) * 2
    }

    #[tokio::test]
    async fn test_runnable_parallel() {
        let double = RunnableLambda::new(|v: Value| {
            Box::pin(async move {
                let n = v.as_i64().unwrap() * 2;
                Ok(Value::from(n))
            })
        });
        let passthrough = RunnablePassthrough;

        let par = RunnableParallel::new(vec![
            ("doubled".into(), Box::new(double) as Box<dyn RunnableCore>),
            (
                "original".into(),
                Box::new(passthrough) as Box<dyn RunnableCore>,
            ),
        ]);

        let result = par.invoke(Value::from(5), None).await.unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("doubled").unwrap(), &Value::from(10));
        assert_eq!(obj.get("original").unwrap(), &Value::from(5));
    }

    #[tokio::test]
    async fn test_sequence_name_default() {
        let seq = RunnableSequence::new(vec![]);
        assert_eq!(seq.name(), "RunnableSequence");
    }

    #[tokio::test]
    async fn test_sequence_name_custom() {
        let seq = RunnableSequence::new(vec![]).with_name("my_chain");
        assert_eq!(seq.name(), "my_chain");
    }
}

//! A channel that reduces values with a binary operator.
//!
//! [`BinaryOperatorAggregate`] applies a user-supplied reducer function
//! cumulatively to combine an existing value with incoming updates.

use crate::channels::traits::BaseChannel;
use crate::error::GraphError;

/// A reducer function that combines two JSON values into one.
pub type ReducerFn =
    Box<dyn Fn(&serde_json::Value, &serde_json::Value) -> serde_json::Value + Send + Sync>;

/// A channel that aggregates values using a binary operator.
///
/// On each update, the reducer is applied left-to-right: starting from the
/// current value (or an initial value), each incoming value is folded in.
pub struct BinaryOperatorAggregate {
    key: String,
    value: Option<serde_json::Value>,
    reducer: ReducerFn,
}

impl BinaryOperatorAggregate {
    /// Creates a new `BinaryOperatorAggregate` channel.
    ///
    /// The `reducer` is called as `reducer(current, incoming)` for each value
    /// in an update batch.
    pub fn new(key: impl Into<String>, reducer: ReducerFn) -> Self {
        Self {
            key: key.into(),
            value: None,
            reducer,
        }
    }

    /// Creates a new channel with an initial value.
    pub fn with_initial(
        key: impl Into<String>,
        initial: serde_json::Value,
        reducer: ReducerFn,
    ) -> Self {
        Self {
            key: key.into(),
            value: Some(initial),
            reducer,
        }
    }
}

impl BaseChannel for BinaryOperatorAggregate {
    fn key(&self) -> &str {
        &self.key
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError> {
        for v in values {
            self.value = Some(match self.value.take() {
                Some(current) => (self.reducer)(&current, &v),
                None => v,
            });
        }
        Ok(())
    }

    fn get(&self) -> Option<&serde_json::Value> {
        self.value.as_ref()
    }

    fn checkpoint(&self) -> serde_json::Value {
        self.value.clone().unwrap_or(serde_json::Value::Null)
    }

    fn restore_checkpoint(&mut self, value: serde_json::Value) {
        if value.is_null() {
            self.value = None;
        } else {
            self.value = Some(value);
        }
    }

    fn consume(&mut self) -> Option<serde_json::Value> {
        self.value.take()
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sum_reducer() -> ReducerFn {
        Box::new(|a, b| {
            let a = a.as_i64().unwrap_or(0);
            let b = b.as_i64().unwrap_or(0);
            serde_json::json!(a + b)
        })
    }

    #[test]
    fn reduces_values() {
        let mut ch = BinaryOperatorAggregate::new("total", sum_reducer());
        ch.update(vec![serde_json::json!(10)]).unwrap();
        ch.update(vec![serde_json::json!(5)]).unwrap();
        assert_eq!(ch.get().unwrap(), &serde_json::json!(15));
    }

    #[test]
    fn reduces_batch() {
        let mut ch = BinaryOperatorAggregate::new("total", sum_reducer());
        ch.update(vec![
            serde_json::json!(1),
            serde_json::json!(2),
            serde_json::json!(3),
        ])
        .unwrap();
        assert_eq!(ch.get().unwrap(), &serde_json::json!(6));
    }

    #[test]
    fn with_initial_value() {
        let mut ch =
            BinaryOperatorAggregate::with_initial("total", serde_json::json!(100), sum_reducer());
        ch.update(vec![serde_json::json!(5)]).unwrap();
        assert_eq!(ch.get().unwrap(), &serde_json::json!(105));
    }

    #[test]
    fn checkpoint_roundtrip() {
        let mut ch = BinaryOperatorAggregate::new("total", sum_reducer());
        ch.update(vec![serde_json::json!(42)]).unwrap();
        let cp = ch.checkpoint();
        let mut ch2 = BinaryOperatorAggregate::new("total", sum_reducer());
        ch2.restore_checkpoint(cp);
        assert_eq!(ch2.get().unwrap(), &serde_json::json!(42));
    }
}

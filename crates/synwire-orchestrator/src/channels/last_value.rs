//! A channel that stores the most recent value.
//!
//! [`LastValue`] keeps only the latest value written. If multiple values are
//! supplied in a single superstep, it rejects the update.

use crate::channels::traits::BaseChannel;
use crate::error::GraphError;

/// A channel that retains only the most recently written value.
///
/// Attempting to update with more than one value in a single superstep
/// produces an error.
pub struct LastValue {
    key: String,
    value: Option<serde_json::Value>,
}

impl LastValue {
    /// Creates a new `LastValue` channel with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: None,
        }
    }
}

impl BaseChannel for LastValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError> {
        if values.len() > 1 {
            return Err(GraphError::MultipleValues {
                channel: self.key.clone(),
            });
        }
        if let Some(v) = values.into_iter().next() {
            self.value = Some(v);
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

    #[test]
    fn stores_single_value() {
        let mut ch = LastValue::new("x");
        ch.update(vec![serde_json::json!(42)]).unwrap();
        assert_eq!(ch.get().unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn rejects_multiple_values() {
        let mut ch = LastValue::new("x");
        let result = ch.update(vec![serde_json::json!(1), serde_json::json!(2)]);
        assert!(result.is_err());
    }

    #[test]
    fn overwrites_on_successive_updates() {
        let mut ch = LastValue::new("x");
        ch.update(vec![serde_json::json!(1)]).unwrap();
        ch.update(vec![serde_json::json!(2)]).unwrap();
        assert_eq!(ch.get().unwrap(), &serde_json::json!(2));
    }

    #[test]
    fn checkpoint_roundtrip() {
        let mut ch = LastValue::new("x");
        ch.update(vec![serde_json::json!("hello")]).unwrap();
        let cp = ch.checkpoint();
        let mut ch2 = LastValue::new("x");
        ch2.restore_checkpoint(cp);
        assert_eq!(ch2.get().unwrap(), &serde_json::json!("hello"));
    }

    #[test]
    fn consume_takes_value() {
        let mut ch = LastValue::new("x");
        ch.update(vec![serde_json::json!(99)]).unwrap();
        let v = ch.consume();
        assert_eq!(v.unwrap(), serde_json::json!(99));
        assert!(!ch.is_available());
    }

    #[test]
    fn empty_update_is_noop() {
        let mut ch = LastValue::new("x");
        ch.update(vec![]).unwrap();
        assert!(!ch.is_available());
    }
}

//! A channel that clears after being read.
//!
//! [`EphemeralValue`] stores a single value that is automatically cleared
//! once consumed. Useful for one-shot signals between nodes.

use crate::channels::traits::BaseChannel;
use crate::error::GraphError;

/// A channel whose value is cleared after consumption.
///
/// Unlike [`super::last_value::LastValue`], an ephemeral channel does not
/// persist its value across supersteps unless explicitly restored from a
/// checkpoint.
pub struct EphemeralValue {
    key: String,
    value: Option<serde_json::Value>,
}

impl EphemeralValue {
    /// Creates a new `EphemeralValue` channel with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: None,
        }
    }
}

impl BaseChannel for EphemeralValue {
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
        // Ephemeral values are not persisted in checkpoints.
        serde_json::Value::Null
    }

    fn restore_checkpoint(&mut self, _value: serde_json::Value) {
        // Ephemeral values are never restored from checkpoints.
        self.value = None;
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
    fn stores_and_consumes() {
        let mut ch = EphemeralValue::new("sig");
        ch.update(vec![serde_json::json!(true)]).unwrap();
        assert!(ch.is_available());
        let v = ch.consume().unwrap();
        assert_eq!(v, serde_json::json!(true));
        assert!(!ch.is_available());
    }

    #[test]
    fn checkpoint_is_null() {
        let mut ch = EphemeralValue::new("sig");
        ch.update(vec![serde_json::json!(42)]).unwrap();
        assert_eq!(ch.checkpoint(), serde_json::Value::Null);
    }

    #[test]
    fn rejects_multiple_values() {
        let mut ch = EphemeralValue::new("sig");
        let result = ch.update(vec![serde_json::json!(1), serde_json::json!(2)]);
        assert!(result.is_err());
    }
}

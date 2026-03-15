//! A channel that accepts exactly one value.
//!
//! [`AnyValue`] behaves like [`super::last_value::LastValue`] but is semantically
//! used when any single value from a set of writers is acceptable.

use crate::channels::traits::BaseChannel;
use crate::error::GraphError;

/// A channel that accepts exactly one value per superstep.
///
/// If multiple values are supplied, only the first is kept and the rest
/// are silently discarded. Use this when you need a "pick any" semantic.
pub struct AnyValue {
    key: String,
    value: Option<serde_json::Value>,
}

impl AnyValue {
    /// Creates a new `AnyValue` channel with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: None,
        }
    }
}

impl BaseChannel for AnyValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError> {
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
    fn accepts_single_value() {
        let mut ch = AnyValue::new("v");
        ch.update(vec![serde_json::json!("first")]).unwrap();
        assert_eq!(ch.get().unwrap(), &serde_json::json!("first"));
    }

    #[test]
    fn picks_first_of_many() {
        let mut ch = AnyValue::new("v");
        ch.update(vec![serde_json::json!("a"), serde_json::json!("b")])
            .unwrap();
        assert_eq!(ch.get().unwrap(), &serde_json::json!("a"));
    }

    #[test]
    fn empty_when_no_update() {
        let ch = AnyValue::new("v");
        assert!(!ch.is_available());
    }
}

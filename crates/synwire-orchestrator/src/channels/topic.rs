//! A channel that accumulates values in order.
//!
//! [`Topic`] collects all values written during execution into a JSON array.
//! Useful for message histories and event logs.

use crate::channels::traits::BaseChannel;
use crate::error::GraphError;

/// A channel that accumulates values into an ordered list.
///
/// Each update appends values to the internal list. The channel value is
/// always a JSON array.
pub struct Topic {
    key: String,
    values: Vec<serde_json::Value>,
}

impl Topic {
    /// Creates a new `Topic` channel with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            values: Vec::new(),
        }
    }
}

impl BaseChannel for Topic {
    fn key(&self) -> &str {
        &self.key
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError> {
        self.values.extend(values);
        Ok(())
    }

    fn get(&self) -> Option<&serde_json::Value> {
        // We cannot return a reference to a dynamically constructed array,
        // so Topic stores a cached representation. This is handled via
        // checkpoint instead. For `get`, we return None if empty.
        None
    }

    fn checkpoint(&self) -> serde_json::Value {
        serde_json::Value::Array(self.values.clone())
    }

    fn restore_checkpoint(&mut self, value: serde_json::Value) {
        if let serde_json::Value::Array(arr) = value {
            self.values = arr;
        } else {
            self.values = vec![value];
        }
    }

    fn consume(&mut self) -> Option<serde_json::Value> {
        if self.values.is_empty() {
            None
        } else {
            let taken = std::mem::take(&mut self.values);
            Some(serde_json::Value::Array(taken))
        }
    }

    fn is_available(&self) -> bool {
        !self.values.is_empty()
    }
}

impl Topic {
    /// Returns the accumulated values as a slice.
    pub fn values(&self) -> &[serde_json::Value] {
        &self.values
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_values() {
        let mut ch = Topic::new("msgs");
        ch.update(vec![serde_json::json!("a")]).unwrap();
        ch.update(vec![serde_json::json!("b"), serde_json::json!("c")])
            .unwrap();
        assert_eq!(ch.values().len(), 3);
    }

    #[test]
    fn checkpoint_roundtrip() {
        let mut ch = Topic::new("msgs");
        ch.update(vec![serde_json::json!(1), serde_json::json!(2)])
            .unwrap();
        let cp = ch.checkpoint();
        let mut ch2 = Topic::new("msgs");
        ch2.restore_checkpoint(cp);
        assert_eq!(ch2.values().len(), 2);
    }

    #[test]
    fn consume_drains() {
        let mut ch = Topic::new("msgs");
        ch.update(vec![serde_json::json!("x")]).unwrap();
        let v = ch.consume().unwrap();
        assert_eq!(v, serde_json::json!(["x"]));
        assert!(!ch.is_available());
    }

    #[test]
    fn empty_topic_not_available() {
        let mut ch = Topic::new("msgs");
        assert!(!ch.is_available());
        assert!(ch.consume().is_none());
    }
}

//! A barrier channel that fires when all named triggers are received.
//!
//! [`NamedBarrierValue`] tracks a set of expected trigger names. It becomes
//! available only once every expected name has been received.

use std::collections::HashSet;

use crate::channels::traits::BaseChannel;
use crate::error::GraphError;

/// A channel that becomes available when all named triggers have fired.
///
/// Each update value must be a JSON string matching one of the expected
/// trigger names. The channel becomes available only when all expected
/// names have been received.
pub struct NamedBarrierValue {
    key: String,
    expected: HashSet<String>,
    received: HashSet<String>,
}

impl NamedBarrierValue {
    /// Creates a new `NamedBarrierValue` channel.
    ///
    /// The `expected` set contains the names of all triggers that must fire
    /// before this channel becomes available.
    pub fn new(key: impl Into<String>, expected: HashSet<String>) -> Self {
        Self {
            key: key.into(),
            expected,
            received: HashSet::new(),
        }
    }
}

impl BaseChannel for NamedBarrierValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError> {
        for v in values {
            if let Some(name) = v.as_str() {
                let _inserted = self.received.insert(name.to_owned());
            } else {
                return Err(GraphError::InvalidUpdate {
                    message: format!(
                        "named barrier '{}' expects string values, got {}",
                        self.key, v
                    ),
                });
            }
        }
        Ok(())
    }

    fn get(&self) -> Option<&serde_json::Value> {
        // Barrier channels have no meaningful "value" to reference.
        None
    }

    fn checkpoint(&self) -> serde_json::Value {
        let received: Vec<serde_json::Value> = self
            .received
            .iter()
            .map(|s| serde_json::Value::String(s.clone()))
            .collect();
        serde_json::Value::Array(received)
    }

    fn restore_checkpoint(&mut self, value: serde_json::Value) {
        self.received.clear();
        if let serde_json::Value::Array(arr) = value {
            for v in arr {
                if let Some(s) = v.as_str() {
                    let _inserted = self.received.insert(s.to_owned());
                }
            }
        }
    }

    fn consume(&mut self) -> Option<serde_json::Value> {
        if self.is_available() {
            self.received.clear();
            Some(serde_json::json!(true))
        } else {
            None
        }
    }

    fn is_available(&self) -> bool {
        self.expected.iter().all(|e| self.received.contains(e))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn barrier(names: &[&str]) -> NamedBarrierValue {
        let expected: HashSet<String> = names.iter().map(|s| (*s).to_owned()).collect();
        NamedBarrierValue::new("gate", expected)
    }

    #[test]
    fn not_available_until_all_received() {
        let mut ch = barrier(&["a", "b"]);
        assert!(!ch.is_available());
        ch.update(vec![serde_json::json!("a")]).unwrap();
        assert!(!ch.is_available());
        ch.update(vec![serde_json::json!("b")]).unwrap();
        assert!(ch.is_available());
    }

    #[test]
    fn consume_resets() {
        let mut ch = barrier(&["x"]);
        ch.update(vec![serde_json::json!("x")]).unwrap();
        let v = ch.consume().unwrap();
        assert_eq!(v, serde_json::json!(true));
        assert!(!ch.is_available());
    }

    #[test]
    fn rejects_non_string() {
        let mut ch = barrier(&["a"]);
        let result = ch.update(vec![serde_json::json!(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn checkpoint_roundtrip() {
        let mut ch = barrier(&["a", "b"]);
        ch.update(vec![serde_json::json!("a")]).unwrap();
        let cp = ch.checkpoint();
        let mut ch2 = barrier(&["a", "b"]);
        ch2.restore_checkpoint(cp);
        assert!(!ch2.is_available());
        ch2.update(vec![serde_json::json!("b")]).unwrap();
        assert!(ch2.is_available());
    }
}

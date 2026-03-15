//! JSON serializer with special handling for `SecretValue` sentinels.
//!
//! `SecretValue` fields are serialized as JSON `null` to prevent accidental
//! leakage of sensitive data into checkpoint storage. On deserialization,
//! `null` values are preserved as-is (the caller is responsible for
//! re-hydrating secrets from a secure store).

use crate::types::CheckpointError;

use super::protocol::SerializerProtocol;

/// JSON-based serializer that redacts `SecretValue` sentinels.
///
/// # Examples
///
/// ```
/// use synwire_checkpoint::serde::json_plus::JsonPlusSerializer;
/// use synwire_checkpoint::serde::protocol::SerializerProtocol;
///
/// let serializer = JsonPlusSerializer;
/// let value = serde_json::json!({"key": "value"});
/// let bytes = serializer.dumps_typed(&value).unwrap();
/// let round_tripped = serializer.loads_typed(&bytes).unwrap();
/// assert_eq!(value, round_tripped);
/// ```
pub struct JsonPlusSerializer;

impl SerializerProtocol for JsonPlusSerializer {
    fn dumps_typed(&self, value: &serde_json::Value) -> Result<Vec<u8>, CheckpointError> {
        let redacted = redact_secrets(value);
        serde_json::to_vec(&redacted).map_err(|e| CheckpointError::Serialization(e.to_string()))
    }

    fn loads_typed(&self, data: &[u8]) -> Result<serde_json::Value, CheckpointError> {
        serde_json::from_slice(data).map_err(|e| CheckpointError::Serialization(e.to_string()))
    }
}

/// Recursively redact any object with `{"__secret__": true}` to `null`.
fn redact_secrets(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("__secret__").and_then(serde_json::Value::as_bool) == Some(true) {
                return serde_json::Value::Null;
            }
            let redacted = map
                .iter()
                .map(|(k, v)| (k.clone(), redact_secrets(v)))
                .collect();
            serde_json::Value::Object(redacted)
        }
        serde_json::Value::Array(arr) => {
            let redacted = arr.iter().map(redact_secrets).collect();
            serde_json::Value::Array(redacted)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    /// T218: `JsonPlusSerializer` round-trip.
    #[test]
    fn round_trip() {
        let serializer = JsonPlusSerializer;
        let value = json!({"key": "value", "nested": {"a": 1}});
        let bytes = serializer.dumps_typed(&value).unwrap();
        let result = serializer.loads_typed(&bytes).unwrap();
        assert_eq!(value, result);
    }

    /// T219: `SecretValue` sentinel handling -- serialize to null, deserialize back.
    #[test]
    fn secret_value_sentinel_redacted() {
        let serializer = JsonPlusSerializer;
        let value = json!({
            "api_key": {"__secret__": true, "value": "sk-1234"},
            "name": "test"
        });
        let bytes = serializer.dumps_typed(&value).unwrap();
        let result = serializer.loads_typed(&bytes).unwrap();
        assert_eq!(result["api_key"], json!(null));
        assert_eq!(result["name"], json!("test"));
    }
}

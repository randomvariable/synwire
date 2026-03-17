//! State trait for agent state management.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// State trait for agent state.
///
/// Agents maintain state that can be serialized and deserialized.
pub trait State: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {
    /// Serialize state to JSON value.
    fn to_value(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    /// Deserialize state from JSON value.
    fn from_value(value: Value) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>;
}

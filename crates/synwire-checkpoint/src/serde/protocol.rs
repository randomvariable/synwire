//! Serializer protocol trait for checkpoint data encoding.

use crate::types::CheckpointError;

/// Protocol for serializing and deserializing checkpoint values.
///
/// Implementations convert between `serde_json::Value` and raw bytes
/// for storage in checkpoint backends.
pub trait SerializerProtocol: Send + Sync {
    /// Serialize a JSON value to bytes.
    fn dumps_typed(&self, value: &serde_json::Value) -> Result<Vec<u8>, CheckpointError>;

    /// Deserialize bytes back into a JSON value.
    fn loads_typed(&self, data: &[u8]) -> Result<serde_json::Value, CheckpointError>;
}

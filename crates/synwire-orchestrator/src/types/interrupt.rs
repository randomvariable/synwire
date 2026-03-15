//! Interrupt types for pausing graph execution.

use serde::{Deserialize, Serialize};

/// An interrupt that pauses graph execution with an associated value.
///
/// Use [`interrupt`] to create an interrupt during node execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interrupt {
    /// The value associated with this interrupt.
    pub value: serde_json::Value,
}

impl Interrupt {
    /// Creates a new interrupt with the given value.
    pub const fn new(value: serde_json::Value) -> Self {
        Self { value }
    }
}

/// Creates an interrupt that pauses graph execution.
///
/// This returns a [`GraphError::Interrupt`](crate::error::GraphError::Interrupt)
/// containing the JSON-serialized interrupt value.
pub fn interrupt(value: &serde_json::Value) -> crate::error::GraphError {
    crate::error::GraphError::Interrupt {
        message: value.to_string(),
    }
}

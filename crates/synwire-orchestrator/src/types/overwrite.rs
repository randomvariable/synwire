//! Overwrite wrapper for channel updates.

use serde::{Deserialize, Serialize};

/// A wrapper that signals the channel should replace its current value
/// rather than applying the normal reduction strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overwrite<V> {
    /// The value to overwrite with.
    pub value: V,
}

impl<V> Overwrite<V> {
    /// Creates a new `Overwrite` with the given value.
    pub const fn new(value: V) -> Self {
        Self { value }
    }
}

//! Errors specific to embedding operations.

/// Errors specific to embedding operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EmbeddingError {
    /// Embedding request failed.
    #[error("embedding failed: {message}")]
    Failed {
        /// Error message.
        message: String,
    },
    /// Dimension mismatch.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimensions.
        expected: usize,
        /// Actual dimensions.
        actual: usize,
    },
    /// Other embedding error.
    #[error("embedding error: {message}")]
    Other {
        /// Error message.
        message: String,
    },
}

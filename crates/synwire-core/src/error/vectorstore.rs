//! Errors specific to vector store operations.

/// Errors specific to vector store operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VectorStoreError {
    /// Document not found.
    #[error("document not found: {id}")]
    NotFound {
        /// Document ID.
        id: String,
    },
    /// Dimension mismatch when inserting.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimensions.
        expected: usize,
        /// Actual dimensions.
        actual: usize,
    },
    /// Other vector store error.
    #[error("vector store error: {message}")]
    Other {
        /// Error message.
        message: String,
    },
}

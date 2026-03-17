//! Error types for the `LanceDB` vector store.

use synwire_core::error::VectorStoreError;

/// Errors that can occur when interacting with a `LanceDB` vector store.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LanceDbError {
    /// A `LanceDB` internal error.
    #[error("LanceDB error: {0}")]
    Lance(#[from] lancedb::Error),

    /// An Arrow data error (schema or array construction failure).
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),

    /// A JSON serialization or deserialization error.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// The embedding model produced a vector with an unexpected number of dimensions.
    #[error("embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected number of dimensions.
        expected: usize,
        /// Actual number of dimensions produced.
        actual: usize,
    },

    /// An error returned by the embedding model during document or query embedding.
    #[error("embedding error: {0}")]
    Embedding(String),

    /// A result column was missing from the `LanceDB` query output.
    #[error("missing column in query result: {0}")]
    MissingColumn(String),
}

impl From<LanceDbError> for synwire_core::error::SynwireError {
    fn from(err: LanceDbError) -> Self {
        Self::VectorStore(VectorStoreError::Other {
            message: err.to_string(),
        })
    }
}

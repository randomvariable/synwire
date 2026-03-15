//! Error types for the Synwire framework.
//!
//! This module provides a structured error hierarchy with [`SynwireError`] as
//! the top-level type, domain-specific error enums for each subsystem, and
//! [`SynwireErrorKind`] for discriminant-based retry and fallback matching.

mod embedding;
mod kind;
mod model;
mod parse;
mod tool;
mod vectorstore;

pub use embedding::EmbeddingError;
pub use kind::SynwireErrorKind;
pub use model::ModelError;
pub use parse::ParseError;
pub use tool::ToolError;
pub use vectorstore::VectorStoreError;

/// Top-level error type for the Synwire framework.
///
/// Wraps domain-specific error types via `#[from]` conversions.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SynwireError {
    /// Model invocation error.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// Prompt formatting error.
    #[error("prompt error: {message}")]
    Prompt {
        /// Error message.
        message: String,
    },
    /// Output parsing error.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Embedding error.
    #[error(transparent)]
    Embedding(#[from] EmbeddingError),
    /// Vector store error.
    #[error(transparent)]
    VectorStore(#[from] VectorStoreError),
    /// Tool invocation error.
    #[error(transparent)]
    Tool(#[from] ToolError),
    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// Graph execution error.
    #[error("graph error: {0}")]
    Graph(Box<dyn std::error::Error + Send + Sync>),
    /// Credential error.
    #[error("credential error: {message}")]
    Credential {
        /// Error message.
        message: String,
    },
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Other error.
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl SynwireError {
    /// Returns the error kind discriminant for this error.
    pub const fn kind(&self) -> SynwireErrorKind {
        match self {
            Self::Model(_) => SynwireErrorKind::Model,
            Self::Prompt { .. } => SynwireErrorKind::Prompt,
            Self::Parse(_) => SynwireErrorKind::Parse,
            Self::Embedding(_) => SynwireErrorKind::Embedding,
            Self::VectorStore(_) => SynwireErrorKind::VectorStore,
            Self::Tool(_) => SynwireErrorKind::Tool,
            Self::Serialization(_) => SynwireErrorKind::Serialization,
            Self::Graph(_) => SynwireErrorKind::Graph,
            Self::Credential { .. } => SynwireErrorKind::Credential,
            Self::Io(_) | Self::Other(_) => SynwireErrorKind::Other,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_model() {
        let err = SynwireError::from(ModelError::Timeout);
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn test_error_display_prompt() {
        let err = SynwireError::Prompt {
            message: "missing variable 'name'".into(),
        };
        assert!(err.to_string().contains("missing variable"));
    }

    #[test]
    fn test_error_from_model_error() {
        let model_err = ModelError::RateLimit { retry_after: None };
        let err: SynwireError = model_err.into();
        assert_eq!(err.kind(), SynwireErrorKind::Model);
    }

    #[test]
    fn test_error_from_tool_error() {
        let tool_err = ToolError::NotFound {
            name: "search".into(),
        };
        let err: SynwireError = tool_err.into();
        assert_eq!(err.kind(), SynwireErrorKind::Tool);
    }

    #[test]
    fn test_error_kind_matching() {
        let err = SynwireError::from(ParseError::ParseFailed {
            message: "bad json".into(),
        });
        assert_eq!(err.kind(), SynwireErrorKind::Parse);
    }
}

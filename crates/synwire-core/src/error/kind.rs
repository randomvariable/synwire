//! Error kind discriminants for retry and fallback matching.

/// Discriminant enum for matching errors without payload.
///
/// Used by `RetryConfig` and `with_fallbacks` to specify which
/// error categories to handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SynwireErrorKind {
    /// Model invocation error.
    Model,
    /// Prompt formatting error.
    Prompt,
    /// Output parsing error.
    Parse,
    /// Embedding error.
    Embedding,
    /// Vector store error.
    VectorStore,
    /// Tool invocation error.
    Tool,
    /// Retry exhausted.
    RetryExhausted,
    /// Serialization error.
    Serialization,
    /// Graph execution error.
    Graph,
    /// Credential error.
    Credential,
    /// Other error.
    Other,
}

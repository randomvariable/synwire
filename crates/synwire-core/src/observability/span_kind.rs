//! Observability span kind enumeration.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The kind of observability span, corresponding to AI framework operations.
///
/// This enum is `#[non_exhaustive]` so that new span kinds can be added in
/// future minor releases without breaking downstream code.
///
/// # Example
///
/// ```
/// use synwire_core::observability::ObservabilitySpanKind;
///
/// let kind = ObservabilitySpanKind::Llm;
/// assert_eq!(kind.as_str(), "llm");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ObservabilitySpanKind {
    /// A large language model invocation.
    Llm,
    /// A chain of operations.
    Chain,
    /// A tool invocation.
    Tool,
    /// An embedding operation.
    Embedding,
    /// A retriever query.
    Retriever,
    /// A graph execution step.
    Graph,
}

impl ObservabilitySpanKind {
    /// Returns the string representation of this span kind.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Llm => "llm",
            Self::Chain => "chain",
            Self::Tool => "tool",
            Self::Embedding => "embedding",
            Self::Retriever => "retriever",
            Self::Graph => "graph",
        }
    }
}

impl fmt::Display for ObservabilitySpanKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn span_kind_display() {
        assert_eq!(ObservabilitySpanKind::Llm.to_string(), "llm");
        assert_eq!(ObservabilitySpanKind::Chain.to_string(), "chain");
        assert_eq!(ObservabilitySpanKind::Tool.to_string(), "tool");
        assert_eq!(ObservabilitySpanKind::Embedding.to_string(), "embedding");
        assert_eq!(ObservabilitySpanKind::Retriever.to_string(), "retriever");
        assert_eq!(ObservabilitySpanKind::Graph.to_string(), "graph");
    }

    #[test]
    fn span_kind_serialization_roundtrip() {
        let kind = ObservabilitySpanKind::Llm;
        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: ObservabilitySpanKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, deserialized);
    }
}

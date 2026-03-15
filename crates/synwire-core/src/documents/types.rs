//! Document type definition.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A retrievable piece of content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Optional document identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The document's text content.
    pub page_content: String,
    /// Metadata associated with the document.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl Document {
    /// Creates a new document with the given content.
    pub fn new(page_content: impl Into<String>) -> Self {
        Self {
            id: None,
            page_content: page_content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Creates a new document with content and metadata.
    pub fn with_metadata(
        page_content: impl Into<String>,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            id: None,
            page_content: page_content.into(),
            metadata,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_document_construction() {
        let doc = Document::new("Hello world");
        assert_eq!(doc.page_content, "Hello world");
        assert!(doc.metadata.is_empty());
        assert!(doc.id.is_none());
    }

    #[test]
    fn test_document_with_metadata() {
        let mut metadata = HashMap::new();
        let _ = metadata.insert("source".into(), Value::String("test".into()));
        let doc = Document::with_metadata("content", metadata);
        assert_eq!(doc.page_content, "content");
        assert_eq!(
            doc.metadata.get("source").unwrap(),
            &Value::String("test".into())
        );
    }

    #[test]
    fn test_document_serde_roundtrip() {
        let mut metadata = HashMap::new();
        let _ = metadata.insert("key".into(), Value::Number(42.into()));
        let doc = Document {
            id: Some("doc_1".into()),
            page_content: "Hello".into(),
            metadata,
        };
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id.as_deref(), Some("doc_1"));
        assert_eq!(deserialized.page_content, "Hello");
        assert_eq!(
            deserialized.metadata.get("key").unwrap(),
            &Value::Number(42.into())
        );
    }
}

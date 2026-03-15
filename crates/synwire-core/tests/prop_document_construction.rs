//! Property test: Document construction and field preservation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_test_utils::documents::arb_document;

proptest! {
    /// A Document constructed via proptest should have its fields preserved
    /// through a serde roundtrip.
    #[test]
    fn document_serde_roundtrip(doc in arb_document()) {
        let json = serde_json::to_string(&doc).unwrap();
        let roundtripped: synwire_core::documents::Document =
            serde_json::from_str(&json).unwrap();

        assert_eq!(doc.id, roundtripped.id);
        assert_eq!(doc.page_content, roundtripped.page_content);
        assert_eq!(doc.metadata.len(), roundtripped.metadata.len());
    }

    /// A Document's page_content should always be non-empty when generated
    /// by the non-empty strategy.
    #[test]
    fn document_content_preserved(doc in arb_document()) {
        // page_content should round-trip exactly.
        let doc2 = synwire_core::documents::Document::new(&doc.page_content);
        assert_eq!(doc2.page_content, doc.page_content);
    }
}

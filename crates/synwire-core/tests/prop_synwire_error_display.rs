//! Property test: `SynwireError` Display implementations produce non-empty output.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_core::error::{
    EmbeddingError, ModelError, ParseError, SynwireError, ToolError, VectorStoreError,
};

/// Strategy for generating arbitrary `SynwireError` variants.
fn arb_synwire_error() -> impl Strategy<Value = SynwireError> {
    prop_oneof![
        ".{1,50}".prop_map(|msg| SynwireError::Prompt { message: msg }),
        ".{1,50}".prop_map(|msg| SynwireError::Credential { message: msg }),
        ".{1,50}".prop_map(|_| SynwireError::Model(ModelError::Timeout)),
        ".{1,50}".prop_map(|msg| SynwireError::Model(ModelError::Other { message: msg })),
        ".{1,50}".prop_map(|msg| SynwireError::Parse(ParseError::ParseFailed { message: msg })),
        ".{1,50}".prop_map(|msg| SynwireError::Embedding(EmbeddingError::Failed { message: msg })),
        ".{1,50}"
            .prop_map(|msg| SynwireError::VectorStore(VectorStoreError::Other { message: msg })),
        ".{1,50}".prop_map(|msg| SynwireError::Tool(ToolError::InvocationFailed { message: msg })),
    ]
}

proptest! {
    /// Every SynwireError variant should produce a non-empty Display string.
    #[test]
    fn error_display_is_non_empty(err in arb_synwire_error()) {
        let display = err.to_string();
        assert!(!display.is_empty(), "error display should not be empty");
    }

    /// The error kind should be consistent for a given variant.
    #[test]
    fn error_kind_is_consistent(err in arb_synwire_error()) {
        let kind1 = err.kind();
        let kind2 = err.kind();
        assert_eq!(kind1, kind2, "error kind should be deterministic");
    }
}

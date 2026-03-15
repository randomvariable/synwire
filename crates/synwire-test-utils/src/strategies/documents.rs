//! Proptest strategies for [`Document`].

use std::collections::HashMap;

use proptest::prelude::*;
use serde_json::Value;
use synwire_core::documents::Document;

/// Strategy for generating arbitrary [`Document`] instances.
pub fn arb_document() -> impl Strategy<Value = Document> {
    (
        proptest::option::of("[a-z0-9_-]{1,32}"),
        ".{1,200}",
        arb_metadata(),
    )
        .prop_map(|(id, page_content, metadata)| Document {
            id,
            page_content,
            metadata,
        })
}

/// Strategy for generating document metadata.
pub fn arb_metadata() -> impl Strategy<Value = HashMap<String, Value>> {
    prop::collection::hash_map(
        "[a-z_]{1,16}",
        prop_oneof![
            ".*".prop_map(Value::String),
            any::<i64>().prop_map(|n| Value::Number(n.into())),
            any::<bool>().prop_map(Value::Bool),
        ],
        0..=5,
    )
}

/// Strategy for generating a non-empty document (always has content).
pub fn arb_non_empty_document() -> impl Strategy<Value = Document> {
    ".{1,200}".prop_map(Document::new)
}

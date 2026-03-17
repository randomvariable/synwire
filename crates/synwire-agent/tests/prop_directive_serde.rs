//! Property tests: `Directive` serialises and deserialises without loss.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_test_utils::strategies::agents::arb_directive;

proptest! {
    /// Serialising a `Directive` to JSON and immediately deserialising it
    /// must produce a value that round-trips identically (same JSON string).
    #[test]
    fn directive_serde_roundtrip(directive in arb_directive()) {
        let json1 = serde_json::to_string(&directive)
            .expect("first serialisation must not fail");
        let recovered: synwire_core::agents::directive::Directive =
            serde_json::from_str(&json1)
            .expect("deserialisation must not fail");
        let json2 = serde_json::to_string(&recovered)
            .expect("second serialisation must not fail");
        prop_assert_eq!(json1, json2);
    }

    /// The `type` tag written by serde must survive a round-trip unchanged.
    #[test]
    fn directive_type_tag_preserved(directive in arb_directive()) {
        let val: serde_json::Value = serde_json::to_value(&directive)
            .expect("to_value must not fail");
        let type_tag = val["type"].as_str().expect("'type' field must be a string").to_owned();

        let recovered: synwire_core::agents::directive::Directive =
            serde_json::from_value(val).expect("from_value must not fail");
        let val2 = serde_json::to_value(&recovered).expect("to_value must not fail");
        prop_assert_eq!(&type_tag, val2["type"].as_str().expect("'type' field must be a string"));
    }
}

//! Property tests: `AgentEvent` serialises and deserialises without loss.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_test_utils::strategies::agents::arb_agent_event;

proptest! {
    /// Serialising an `AgentEvent` to JSON and immediately deserialising it
    /// must produce a value that round-trips identically (same JSON string).
    #[test]
    fn agent_event_serde_roundtrip(event in arb_agent_event()) {
        let json1 = serde_json::to_string(&event)
            .expect("first serialisation must not fail");
        let recovered: synwire_core::agents::streaming::AgentEvent =
            serde_json::from_str(&json1)
            .expect("deserialisation must not fail");
        let json2 = serde_json::to_string(&recovered)
            .expect("second serialisation must not fail");
        prop_assert_eq!(json1, json2);
    }

    /// The `type` tag must survive a round-trip unchanged.
    #[test]
    fn agent_event_type_tag_preserved(event in arb_agent_event()) {
        let val: serde_json::Value = serde_json::to_value(&event)
            .expect("to_value must not fail");
        let type_tag = val["type"].as_str().expect("'type' field must be a string").to_owned();

        let recovered: synwire_core::agents::streaming::AgentEvent =
            serde_json::from_value(val).expect("from_value must not fail");
        let val2 = serde_json::to_value(&recovered).expect("to_value must not fail");
        prop_assert_eq!(&type_tag, val2["type"].as_str().expect("'type' field must be a string"));
    }
}

//! Property test: Message serialization roundtrip.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_test_utils::messages::arb_message;

proptest! {
    /// Serializing a Message to JSON and deserializing it back should
    /// preserve the message type and content text.
    #[test]
    fn message_serde_roundtrip(msg in arb_message()) {
        let json = serde_json::to_string(&msg).unwrap();
        let roundtripped: synwire_core::messages::Message =
            serde_json::from_str(&json).unwrap();

        // The message type should survive the roundtrip.
        assert_eq!(msg.message_type(), roundtripped.message_type());

        // The text content should survive the roundtrip.
        assert_eq!(msg.content().as_text(), roundtripped.content().as_text());
    }
}

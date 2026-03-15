//! Property test: Checkpoint serde backwards compatibility.
//!
//! Ensures that checkpoints with `format_version` "1.0" always serialize
//! and deserialize correctly, as a guard against breaking format changes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_test_utils::checkpoints::arb_checkpoint;

proptest! {
    /// A Checkpoint should survive a serde roundtrip and retain its format_version.
    #[test]
    fn checkpoint_serde_roundtrip(cp in arb_checkpoint()) {
        let json = serde_json::to_string(&cp).unwrap();
        let roundtripped: synwire_checkpoint::types::Checkpoint =
            serde_json::from_str(&json).unwrap();

        assert_eq!(cp.id, roundtripped.id);
        assert_eq!(cp.format_version, roundtripped.format_version);
        assert_eq!(cp.channel_values.len(), roundtripped.channel_values.len());
        assert_eq!(cp.channel_versions.len(), roundtripped.channel_versions.len());
        assert_eq!(cp.pending_writes.len(), roundtripped.pending_writes.len());
    }

    /// format_version should always be "1.0" for checkpoints created with the
    /// current constructor.
    #[test]
    fn checkpoint_format_version_is_stable(id in "[a-z0-9-]{8,32}") {
        let cp = synwire_checkpoint::types::Checkpoint::new(id);
        assert_eq!(cp.format_version, "1.0");
    }

    /// A serialized checkpoint should be deserializable even when fields are
    /// reordered (JSON is unordered).
    #[test]
    fn checkpoint_field_order_independent(cp in arb_checkpoint()) {
        let value = serde_json::to_value(&cp).unwrap();
        // Re-serialize from Value (potentially different field order).
        let json = serde_json::to_string(&value).unwrap();
        let roundtripped: synwire_checkpoint::types::Checkpoint =
            serde_json::from_str(&json).unwrap();

        assert_eq!(cp.id, roundtripped.id);
    }
}

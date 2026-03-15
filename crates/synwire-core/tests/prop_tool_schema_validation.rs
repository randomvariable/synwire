//! Property test: `ToolSchema` serde roundtrip.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_test_utils::tools::arb_tool_schema;

proptest! {
    /// A ToolSchema should survive a serde roundtrip, preserving name,
    /// description, and parameters structure.
    #[test]
    fn tool_schema_serde_roundtrip(schema in arb_tool_schema()) {
        let json = serde_json::to_string(&schema).unwrap();
        let roundtripped: synwire_core::tools::ToolSchema =
            serde_json::from_str(&json).unwrap();

        assert_eq!(schema.name, roundtripped.name);
        assert_eq!(schema.description, roundtripped.description);
        assert_eq!(schema.parameters, roundtripped.parameters);
    }

    /// A ToolSchema's parameters should always have `type: "object"` at the top level.
    #[test]
    fn tool_schema_has_object_type(schema in arb_tool_schema()) {
        let type_val = schema.parameters.get("type");
        assert_eq!(
            type_val.and_then(|v| v.as_str()),
            Some("object"),
            "parameters should have type=object"
        );
    }
}

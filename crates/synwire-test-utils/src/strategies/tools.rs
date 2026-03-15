//! Proptest strategies for tool types.

use proptest::prelude::*;
use serde_json::{Value, json};
use synwire_core::tools::{ToolOutput, ToolResult, ToolSchema};

/// Strategy for generating arbitrary [`ToolSchema`].
pub fn arb_tool_schema() -> impl Strategy<Value = ToolSchema> {
    ("[a-z_]{1,24}", ".{1,100}", arb_json_schema()).prop_map(|(name, description, parameters)| {
        ToolSchema {
            name,
            description,
            parameters,
        }
    })
}

/// Strategy for generating a valid JSON Schema object for tool parameters.
pub fn arb_json_schema() -> impl Strategy<Value = Value> {
    prop::collection::vec(
        (
            "[a-z_]{1,12}",
            prop_oneof![Just("string"), Just("number"), Just("boolean")],
        ),
        0..=4,
    )
    .prop_map(|fields| {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        for (name, type_name) in &fields {
            let _ = properties.insert(name.clone(), json!({ "type": type_name }));
            required.push(json!(name));
        }
        json!({
            "type": "object",
            "properties": properties,
            "required": required,
        })
    })
}

/// Strategy for generating arbitrary [`ToolOutput`].
pub fn arb_tool_output() -> impl Strategy<Value = ToolOutput> {
    (".{0,200}", proptest::option::of(arb_simple_json()))
        .prop_map(|(content, artifact)| ToolOutput { content, artifact })
}

/// Strategy for generating arbitrary [`ToolResult`].
pub fn arb_tool_result() -> impl Strategy<Value = ToolResult> {
    prop_oneof![
        arb_simple_json().prop_map(|content| ToolResult::Success { content }),
        ".{1,100}".prop_map(|message| ToolResult::Error { message }),
        ".{1,100}".prop_map(|message| ToolResult::Retry { message }),
    ]
}

/// Strategy for generating simple JSON values.
fn arb_simple_json() -> impl Strategy<Value = Value> {
    prop_oneof![
        ".*".prop_map(Value::String),
        any::<i64>().prop_map(|n| Value::Number(n.into())),
        any::<bool>().prop_map(Value::Bool),
    ]
}

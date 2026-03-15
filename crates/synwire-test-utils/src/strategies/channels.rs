//! Proptest strategies for channel updates.

use proptest::prelude::*;
use serde_json::Value;

/// Strategy for generating a single channel update value.
pub fn arb_channel_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        ".*".prop_map(Value::String),
        any::<i64>().prop_map(|n| Value::Number(n.into())),
        any::<bool>().prop_map(Value::Bool),
        Just(Value::Null),
    ]
}

/// Strategy for generating a batch of channel update values.
pub fn arb_channel_update_batch(max_size: usize) -> impl Strategy<Value = Vec<Value>> {
    prop::collection::vec(arb_channel_value(), 0..=max_size)
}

/// Strategy for generating a single-element update (for `LastValue` channels).
pub fn arb_single_channel_update() -> impl Strategy<Value = Vec<Value>> {
    arb_channel_value().prop_map(|v| vec![v])
}

/// Strategy for generating a channel key name.
pub fn arb_channel_key() -> impl Strategy<Value = String> {
    "[a-z_]{1,16}"
}

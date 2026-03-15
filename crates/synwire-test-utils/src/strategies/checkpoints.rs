//! Proptest strategies for checkpoint types.

use std::collections::HashMap;

use proptest::prelude::*;
use serde_json::{Value, json};
use synwire_checkpoint::types::{
    ChannelVersion, Checkpoint, CheckpointConfig, CheckpointMetadata, CheckpointSource,
    PendingWrite,
};

/// Strategy for generating arbitrary [`Checkpoint`].
pub fn arb_checkpoint() -> impl Strategy<Value = Checkpoint> {
    (
        "[a-z0-9-]{8,32}",
        arb_channel_values(),
        arb_channel_versions(),
        prop::collection::vec(arb_pending_write(), 0..=3),
    )
        .prop_map(
            |(id, channel_values, channel_versions, pending_writes)| Checkpoint {
                id,
                channel_values,
                channel_versions,
                pending_writes,
                format_version: "1.0".into(),
            },
        )
}

/// Strategy for generating arbitrary channel values.
pub fn arb_channel_values() -> impl Strategy<Value = HashMap<String, Value>> {
    prop::collection::hash_map(
        "[a-z_]{1,12}",
        prop_oneof![
            ".*".prop_map(Value::String),
            any::<i64>().prop_map(|n| Value::Number(n.into())),
            Just(json!([])),
        ],
        0..=5,
    )
}

/// Strategy for generating arbitrary channel versions.
pub fn arb_channel_versions() -> impl Strategy<Value = HashMap<String, ChannelVersion>> {
    prop::collection::hash_map(
        "[a-z_]{1,12}",
        any::<u64>().prop_map(|v| ChannelVersion { version: v }),
        0..=5,
    )
}

/// Strategy for generating arbitrary [`PendingWrite`].
pub fn arb_pending_write() -> impl Strategy<Value = PendingWrite> {
    ("[a-z_]{1,12}", ".*".prop_map(Value::String))
        .prop_map(|(channel, value)| PendingWrite { channel, value })
}

/// Strategy for generating arbitrary [`CheckpointConfig`].
pub fn arb_checkpoint_config() -> impl Strategy<Value = CheckpointConfig> {
    ("[a-z0-9-]{8,32}", proptest::option::of("[a-z0-9-]{8,32}")).prop_map(
        |(thread_id, checkpoint_id)| CheckpointConfig {
            thread_id,
            checkpoint_id,
        },
    )
}

/// Strategy for generating arbitrary [`CheckpointMetadata`].
pub fn arb_checkpoint_metadata() -> impl Strategy<Value = CheckpointMetadata> {
    (
        arb_checkpoint_source(),
        0i64..1000,
        arb_channel_values(),
        prop::collection::hash_map("[a-z_]{1,8}", "[a-z0-9-]{8,32}", 0..=2),
    )
        .prop_map(|(source, step, writes, parents)| CheckpointMetadata {
            source,
            step,
            writes,
            parents,
        })
}

/// Strategy for generating arbitrary [`CheckpointSource`].
pub fn arb_checkpoint_source() -> impl Strategy<Value = CheckpointSource> {
    prop_oneof![
        Just(CheckpointSource::Input),
        Just(CheckpointSource::Loop),
        Just(CheckpointSource::Update),
    ]
}

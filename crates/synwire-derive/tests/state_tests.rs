//! Integration tests for the `#[derive(State)]` derive macro.

#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use synwire_derive::State;
use synwire_orchestrator::graph::state::State as _;

#[derive(State, Clone, serde::Serialize, serde::Deserialize, Default)]
struct SimpleState {
    current_step: String,
    score: f64,
}

#[derive(State, Clone, serde::Serialize, serde::Deserialize, Default)]
struct MixedState {
    #[reducer(topic)]
    messages: Vec<String>,
    #[reducer(last_value)]
    status: String,
    counter: i32,
}

#[derive(State, Clone, serde::Serialize, serde::Deserialize, Default)]
struct EmptyState {}

#[test]
fn simple_state_produces_last_value_channels() {
    let channels = SimpleState::channels();
    assert_eq!(channels.len(), 2);

    let names: Vec<&str> = channels.iter().map(|(name, _)| name.as_str()).collect();
    assert!(names.contains(&"current_step"));
    assert!(names.contains(&"score"));

    // All channels should be LastValue (single-value semantics).
    for (_, ch) in &channels {
        assert!(ch.get().is_none());
    }
}

#[test]
fn mixed_state_channel_types() {
    let mut channels = MixedState::channels();
    assert_eq!(channels.len(), 3);

    // Find the "messages" channel (Topic) and verify it accepts multiple values.
    let messages_idx = channels
        .iter()
        .position(|(name, _)| name == "messages")
        .expect("messages channel should exist");

    let (_, ref mut messages_ch) = channels[messages_idx];
    messages_ch
        .update(vec![serde_json::json!("hello"), serde_json::json!("world")])
        .expect("topic should accept multiple values");
    assert!(messages_ch.is_available());

    // Find a LastValue channel and verify it rejects multiple values.
    let status_idx = channels
        .iter()
        .position(|(name, _)| name == "status")
        .expect("status channel should exist");

    let (_, ref mut status_ch) = channels[status_idx];
    let result = status_ch.update(vec![serde_json::json!("a"), serde_json::json!("b")]);
    assert!(result.is_err(), "LastValue should reject multiple values");
}

#[test]
fn empty_state_produces_no_channels() {
    let channels = EmptyState::channels();
    assert!(channels.is_empty());
}

#[test]
fn channel_keys_match_field_names() {
    let channels = MixedState::channels();
    for (name, ch) in &channels {
        assert_eq!(name, ch.key(), "channel key should match field name");
    }
}

#[test]
fn last_value_channel_checkpoint_roundtrip() {
    let mut channels = SimpleState::channels();
    let (_, ref mut ch) = channels[0];

    ch.update(vec![serde_json::json!("test_value")])
        .expect("update should succeed");
    let checkpoint = ch.checkpoint();
    assert_eq!(checkpoint, serde_json::json!("test_value"));
}

/// T016: derive generates `impl State` with correct `channels()`.
#[test]
fn t016_derive_generates_impl_state_with_channels() {
    // Verify the trait method is callable (proves impl State was generated).
    let channels = SimpleState::channels();
    assert_eq!(channels.len(), 2);

    let names: Vec<&str> = channels.iter().map(|(name, _)| name.as_str()).collect();
    assert!(names.contains(&"current_step"));
    assert!(names.contains(&"score"));
}

/// T017: derive generates `from_channels()` that deserialises correctly.
#[test]
fn t017_derive_generates_from_channels() {
    let mut channels = SimpleState::channels();

    // Populate the channels with values.
    for (name, ch) in &mut channels {
        match name.as_str() {
            "current_step" => {
                ch.update(vec![serde_json::json!("step_one")])
                    .expect("update should succeed");
            }
            "score" => {
                ch.update(vec![serde_json::json!(42.5)])
                    .expect("update should succeed");
            }
            _ => unreachable!("unexpected channel: {name}"),
        }
    }

    // Build a HashMap from the channels vec.
    let channel_map: std::collections::HashMap<
        String,
        Box<dyn synwire_orchestrator::channels::BaseChannel>,
    > = channels.into_iter().collect();

    let state = SimpleState::from_channels(&channel_map).expect("from_channels should succeed");
    assert_eq!(state.current_step, "step_one");
    assert!((state.score - 42.5).abs() < f64::EPSILON);
}

/// T018: derive with `#[reducer(topic)]` maps to Topic channel.
#[test]
fn t018_derive_with_reducer_topic() {
    let mut channels = MixedState::channels();

    // The "messages" field should be a Topic channel (accepts multiple values).
    let messages_idx = channels
        .iter()
        .position(|(name, _)| name == "messages")
        .expect("messages channel should exist");

    let (_, ref mut messages_ch) = channels[messages_idx];
    messages_ch
        .update(vec![serde_json::json!("msg1"), serde_json::json!("msg2")])
        .expect("topic channel should accept multiple values");

    // The "status" field should be a LastValue channel.
    let status_idx = channels
        .iter()
        .position(|(name, _)| name == "status")
        .expect("status channel should exist");

    let (_, ref mut status_ch) = channels[status_idx];
    status_ch
        .update(vec![serde_json::json!("active")])
        .expect("last_value should accept single value");

    // Counter channel
    let counter_idx = channels
        .iter()
        .position(|(name, _)| name == "counter")
        .expect("counter channel should exist");

    let (_, ref mut counter_ch) = channels[counter_idx];
    counter_ch
        .update(vec![serde_json::json!(7)])
        .expect("last_value should accept single value");

    // Build map and reconstruct.
    let channel_map: std::collections::HashMap<
        String,
        Box<dyn synwire_orchestrator::channels::BaseChannel>,
    > = channels.into_iter().collect();

    let state = MixedState::from_channels(&channel_map).expect("from_channels should succeed");
    assert_eq!(state.status, "active");
    assert_eq!(state.counter, 7);
}

//! Property test: Topic channel accumulates all written values.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_orchestrator::channels::Topic;
use synwire_orchestrator::channels::traits::BaseChannel;

proptest! {
    /// After multiple updates, Topic should contain the concatenation of all values.
    #[test]
    fn topic_accumulates_all_values(
        batches in prop::collection::vec(
            prop::collection::vec(
                any::<i64>().prop_map(|n| serde_json::json!(n)),
                1..=5,
            ),
            1..=5,
        )
    ) {
        let mut channel = Topic::new("msgs");
        let mut expected_count = 0usize;

        for batch in &batches {
            expected_count += batch.len();
            channel.update(batch.clone()).unwrap();
        }

        assert_eq!(
            channel.values().len(),
            expected_count,
            "topic should contain all accumulated values"
        );
    }

    /// Consuming a Topic should drain it, leaving it empty.
    #[test]
    fn topic_consume_drains(
        values in prop::collection::vec(
            any::<i64>().prop_map(|n| serde_json::json!(n)),
            1..=10,
        )
    ) {
        let mut channel = Topic::new("msgs");
        channel.update(values).unwrap();

        let consumed = channel.consume();
        assert!(consumed.is_some(), "non-empty topic should produce a value on consume");
        assert!(!channel.is_available(), "topic should be empty after consume");
    }
}

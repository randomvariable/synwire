//! Property test: `LastValue` channel always stores the most recent value.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_orchestrator::channels::LastValue;
use synwire_orchestrator::channels::traits::BaseChannel;

proptest! {
    /// After N sequential single-value updates, the channel should hold
    /// the last value written.
    #[test]
    fn last_value_stores_final_update(
        values in prop::collection::vec(
            prop_oneof![
                any::<i64>().prop_map(|n| serde_json::json!(n)),
                ".*".prop_map(|s| serde_json::json!(s)),
            ],
            1..=10,
        )
    ) {
        let mut channel = LastValue::new("test");
        let expected = values.last().unwrap().clone();

        for v in values {
            channel.update(vec![v]).unwrap();
        }

        assert_eq!(
            channel.get().unwrap(),
            &expected,
            "channel should hold the last written value"
        );
    }

    /// Updating with more than one value should return an error.
    #[test]
    fn last_value_rejects_multiple(
        v1 in any::<i64>().prop_map(|n| serde_json::json!(n)),
        v2 in any::<i64>().prop_map(|n| serde_json::json!(n)),
    ) {
        let mut channel = LastValue::new("test");
        let result = channel.update(vec![v1, v2]);
        assert!(result.is_err(), "should reject multiple values");
    }
}

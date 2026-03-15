//! Property test: Channel checkpoint/restore roundtrip preserves state.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_orchestrator::channels::LastValue;
use synwire_orchestrator::channels::Topic;
use synwire_orchestrator::channels::traits::BaseChannel;

proptest! {
    /// LastValue checkpoint/restore should preserve the stored value.
    #[test]
    fn last_value_checkpoint_roundtrip(
        value in any::<i64>().prop_map(|n| serde_json::json!(n)),
    ) {
        let mut ch = LastValue::new("test");
        ch.update(vec![value.clone()]).unwrap();

        let cp = ch.checkpoint();

        let mut ch2 = LastValue::new("test");
        ch2.restore_checkpoint(cp);

        assert_eq!(ch2.get().unwrap(), &value);
    }

    /// Topic checkpoint/restore should preserve accumulated values.
    #[test]
    fn topic_checkpoint_roundtrip(
        values in prop::collection::vec(
            any::<i64>().prop_map(|n| serde_json::json!(n)),
            1..=10,
        ),
    ) {
        let mut ch = Topic::new("msgs");
        ch.update(values.clone()).unwrap();

        let cp = ch.checkpoint();

        let mut ch2 = Topic::new("msgs");
        ch2.restore_checkpoint(cp);

        assert_eq!(ch2.values().len(), values.len());
        for (a, b) in ch2.values().iter().zip(values.iter()) {
            assert_eq!(a, b);
        }
    }
}

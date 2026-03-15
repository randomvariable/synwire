//! Property test: `GraphSend` serde roundtrip preserves all fields.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_orchestrator::types::GraphSend;

proptest! {
    /// GraphSend should survive a serde roundtrip.
    #[test]
    fn graph_send_serde_roundtrip(
        node in "[a-z_]{1,16}",
        value in prop_oneof![
            any::<i64>().prop_map(|n| serde_json::json!(n)),
            ".*".prop_map(|s| serde_json::json!(s)),
            any::<bool>().prop_map(|b| serde_json::json!(b)),
        ],
    ) {
        let send = GraphSend::new(node.clone(), value.clone());
        let json = serde_json::to_string(&send).unwrap();
        let roundtripped: GraphSend = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtripped.node, node);
        assert_eq!(roundtripped.arg, value);
    }

    /// Multiple GraphSend instances to different nodes should all roundtrip correctly.
    #[test]
    fn multiple_sends_roundtrip(
        sends in prop::collection::vec(
            ("[a-z_]{1,8}", any::<i64>().prop_map(|n| serde_json::json!(n))),
            1..=5,
        ),
    ) {
        let graph_sends: Vec<GraphSend> = sends
            .iter()
            .map(|(node, value)| GraphSend::new(node.as_str(), value.clone()))
            .collect();

        let json = serde_json::to_string(&graph_sends).unwrap();
        let roundtripped: Vec<GraphSend> = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtripped.len(), graph_sends.len());
        for (original, rt) in graph_sends.iter().zip(roundtripped.iter()) {
            assert_eq!(original.node, rt.node);
            assert_eq!(original.arg, rt.arg);
        }
    }
}

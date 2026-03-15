//! Property test: Graph execution is deterministic given the same input.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unused_results)]

use proptest::prelude::*;
use synwire_orchestrator::graph::{StateGraph, ValueState};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Running the same compiled graph twice with identical input should
    /// produce identical output.
    #[test]
    fn deterministic_execution(input_value in any::<i64>()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Build a simple identity graph.
            let build = || {
                let mut graph = StateGraph::<ValueState>::new();
                graph.add_node(
                    "identity",
                    Box::new(|state: ValueState| Box::pin(async move { Ok(state) })),
                ).unwrap();
                graph.set_entry_point("identity");
                graph.set_finish_point("identity");
                graph.compile().unwrap()
            };

            let input = ValueState(serde_json::json!({ "value": input_value }));
            let g1 = build();
            let g2 = build();

            let result1 = g1.invoke(input.clone()).await.unwrap();
            let result2 = g2.invoke(input).await.unwrap();

            assert_eq!(result1.0, result2.0, "deterministic graph should produce same output");
        });
    }
}

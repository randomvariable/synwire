//! Property test: Conditional edges route execution correctly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unused_results)]

use std::collections::HashMap;

use proptest::prelude::*;
use synwire_orchestrator::graph::{StateGraph, ValueState};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// A conditional edge should route to the correct branch based on state.
    #[test]
    fn conditional_routes_to_correct_branch(branch_idx in 0usize..3) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let branch_names = ["branch_a", "branch_b", "branch_c"];
            let target = branch_names[branch_idx];

            let mut graph = StateGraph::<ValueState>::new();

            // Router node that sets the route field.
            graph.add_node(
                "router",
                Box::new(move |mut state: ValueState| {
                    Box::pin(async move {
                        if let Some(obj) = state.0.as_object_mut() {
                            obj.insert("visited_router".into(), serde_json::json!(true));
                        }
                        Ok(state)
                    })
                }),
            ).unwrap();

            // Branch nodes.
            for &name in &branch_names {
                let name_owned = name.to_owned();
                graph.add_node(
                    name,
                    Box::new(move |mut state: ValueState| {
                        let tag = name_owned.clone();
                        Box::pin(async move {
                            if let Some(obj) = state.0.as_object_mut() {
                                obj.insert("branch".into(), serde_json::json!(tag));
                            }
                            Ok(state)
                        })
                    }),
                ).unwrap();
                graph.set_finish_point(name);
            }

            graph.set_entry_point("router");

            let mapping: HashMap<String, String> = branch_names
                .iter()
                .map(|&n| (n.to_owned(), n.to_owned()))
                .collect();

            graph.add_conditional_edges(
                "router",
                Box::new(move |state: &ValueState| {
                    // Route based on "route" field in the input.
                    state
                        .0
                        .get("route")
                        .and_then(|v| v.as_str())
                        .unwrap_or("branch_a")
                        .to_owned()
                }),
                mapping,
            );

            let input = ValueState(serde_json::json!({ "route": target }));
            let result = graph.compile().unwrap().invoke(input).await.unwrap();

            assert_eq!(
                result.0.get("branch").and_then(|v| v.as_str()),
                Some(target),
                "should have routed to {target}"
            );
        });
    }
}

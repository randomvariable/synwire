//! Property test: Graph compilation validates topology.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unused_results)]

use proptest::prelude::*;
use synwire_orchestrator::graph::{StateGraph, ValueState};

proptest! {
    /// A graph with a valid linear topology should compile successfully.
    #[test]
    fn linear_graph_compiles(node_count in 1usize..=5) {
        let names: Vec<String> = (0..node_count)
            .map(|i| format!("node_{i}"))
            .collect();

        let mut graph = StateGraph::<ValueState>::new();
        for name in &names {
            graph.add_node(
                name.as_str(),
                Box::new(|state: ValueState| Box::pin(async move { Ok(state) })),
            ).unwrap();
        }

        graph.set_entry_point(&names[0]);

        // Chain the nodes.
        for pair in names.windows(2) {
            graph.add_edge(&pair[0], &pair[1]);
        }
        // Last node to END.
        graph.set_finish_point(names.last().unwrap());

        let compiled = graph.compile();
        assert!(compiled.is_ok(), "valid linear graph should compile");
        let compiled = compiled.unwrap();
        assert_eq!(compiled.entry_point(), names[0]);
    }

    /// A graph without an entry point should fail to compile.
    #[test]
    fn graph_without_entry_point_fails(name in "[a-z]{1,8}") {
        let mut graph = StateGraph::<ValueState>::new();
        graph.add_node(
            name.as_str(),
            Box::new(|state: ValueState| Box::pin(async move { Ok(state) })),
        ).unwrap();
        // No entry point set.
        let result = graph.compile();
        assert!(result.is_err(), "graph without entry point should not compile");
    }

    /// Duplicate node names should be rejected.
    #[test]
    fn duplicate_node_names_rejected(name in "[a-z]{1,8}") {
        let mut graph = StateGraph::<ValueState>::new();
        graph.add_node(
            name.as_str(),
            Box::new(|state: ValueState| Box::pin(async move { Ok(state) })),
        ).unwrap();

        let result = graph.add_node(
            name.as_str(),
            Box::new(|state: ValueState| Box::pin(async move { Ok(state) })),
        );
        assert!(result.is_err(), "duplicate node name should be rejected");
    }
}

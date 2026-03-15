//! Proptest strategies for generating valid graph topologies.

use proptest::prelude::*;

/// A description of a simple valid graph topology for property testing.
#[derive(Debug, Clone)]
pub struct GraphTopology {
    /// Ordered list of node names.
    pub nodes: Vec<String>,
    /// List of edges as `(from, to)` pairs, with `"__end__"` as the terminal.
    pub edges: Vec<(String, String)>,
    /// The entry point node name.
    pub entry_point: String,
}

/// Strategy for generating a valid linear graph topology (chain of nodes).
pub fn arb_linear_graph(
    min_nodes: usize,
    max_nodes: usize,
) -> impl Strategy<Value = GraphTopology> {
    prop::collection::vec("[a-z]{1,8}", min_nodes..=max_nodes).prop_map(|raw_names| {
        // Deduplicate by appending index
        let nodes: Vec<String> = raw_names
            .iter()
            .enumerate()
            .map(|(i, name)| format!("{name}_{i}"))
            .collect();

        let mut edges: Vec<(String, String)> = Vec::new();
        for pair in nodes.windows(2) {
            edges.push((pair[0].clone(), pair[1].clone()));
        }
        // Last node goes to END
        if let Some(last) = nodes.last() {
            edges.push((last.clone(), "__end__".into()));
        }

        let entry_point = nodes.first().cloned().unwrap_or_else(|| "node_0".into());

        GraphTopology {
            nodes,
            edges,
            entry_point,
        }
    })
}

/// Strategy for generating a diamond graph topology (fork and join).
pub fn arb_diamond_graph() -> impl Strategy<Value = GraphTopology> {
    (
        "[a-z]{1,6}",
        prop::collection::vec("[a-z]{1,6}", 2..=4),
        "[a-z]{1,6}",
    )
        .prop_map(|(start_name, branch_names, end_name)| {
            let start = format!("{start_name}_start");
            let end = format!("{end_name}_end");
            let branches: Vec<String> = branch_names
                .iter()
                .enumerate()
                .map(|(i, name)| format!("{name}_branch_{i}"))
                .collect();

            let mut nodes = vec![start.clone()];
            nodes.extend(branches.clone());
            nodes.push(end.clone());

            let mut edges: Vec<(String, String)> = Vec::new();
            for branch in &branches {
                edges.push((start.clone(), branch.clone()));
                edges.push((branch.clone(), end.clone()));
            }
            edges.push((end, "__end__".into()));

            GraphTopology {
                nodes,
                edges,
                entry_point: start,
            }
        })
}

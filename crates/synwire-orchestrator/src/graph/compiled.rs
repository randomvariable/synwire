//! Compiled graph and Pregel-style execution engine.
//!
//! A [`CompiledGraph`] is produced by [`super::StateGraph::compile`] and executes
//! the graph via a superstep loop. Each superstep runs the current node's
//! function, then resolves the next node via static or conditional edges.

use std::collections::HashMap;
use std::fmt;

use crate::constants::{DEFAULT_RECURSION_LIMIT, END};
use crate::error::GraphError;
use crate::graph::state::{ConditionFn, NodeFn, State};

/// A compiled, executable state graph.
///
/// Created by [`StateGraph::compile`](super::state::StateGraph::compile).
/// Call [`invoke`](Self::invoke) to run the graph to completion.
pub struct CompiledGraph<S: State> {
    nodes: HashMap<String, NodeFn<S>>,
    edges: HashMap<String, Vec<String>>,
    conditional_edges: HashMap<String, (ConditionFn<S>, HashMap<String, String>)>,
    entry_point: String,
    recursion_limit: usize,
}

impl<S: State> CompiledGraph<S> {
    /// Creates a new compiled graph (called by `StateGraph::compile`).
    pub(crate) fn new(
        nodes: HashMap<String, NodeFn<S>>,
        edges: HashMap<String, Vec<String>>,
        conditional_edges: HashMap<String, (ConditionFn<S>, HashMap<String, String>)>,
        entry_point: String,
    ) -> Self {
        Self {
            nodes,
            edges,
            conditional_edges,
            entry_point,
            recursion_limit: DEFAULT_RECURSION_LIMIT,
        }
    }

    /// Sets the recursion limit for this graph.
    ///
    /// The graph will return [`GraphError::RecursionLimit`] if execution
    /// exceeds this many supersteps.
    #[must_use]
    pub const fn with_recursion_limit(mut self, limit: usize) -> Self {
        self.recursion_limit = limit;
        self
    }

    /// Returns the configured recursion limit.
    pub const fn recursion_limit(&self) -> usize {
        self.recursion_limit
    }

    /// Returns the entry point node name.
    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }

    /// Returns the names of all nodes in the graph.
    pub fn node_names(&self) -> Vec<&str> {
        self.nodes.keys().map(String::as_str).collect()
    }

    /// Executes the graph to completion with the given typed input state.
    ///
    /// Runs a superstep loop: at each step the current node's function is
    /// called with the state, then the next node is resolved via edges.
    /// Execution terminates when `__end__` is reached or the recursion
    /// limit is exceeded.
    ///
    /// # Cancel safety
    ///
    /// This method is **not cancel-safe**. Dropping the future mid-execution
    /// may leave state partially transformed by whatever node was running.
    /// If you need cancellable graph execution, checkpoint state before each
    /// superstep and resume from the checkpoint on cancellation.
    ///
    /// # Errors
    ///
    /// - [`GraphError::RecursionLimit`] if the step count exceeds the limit.
    /// - [`GraphError::TaskNotFound`] if a node referenced by an edge does not exist.
    /// - [`GraphError::CompileError`] if no outgoing edge is found for a node.
    /// - Any error returned by a node function.
    pub async fn invoke(&self, input: S) -> Result<S, GraphError> {
        let mut state = input;
        let mut current_node = self.entry_point.clone();
        let mut steps: usize = 0;

        loop {
            if steps >= self.recursion_limit {
                return Err(GraphError::RecursionLimit {
                    limit: self.recursion_limit,
                });
            }

            if current_node == END {
                break;
            }

            let node_fn =
                self.nodes
                    .get(&current_node)
                    .ok_or_else(|| GraphError::TaskNotFound {
                        name: current_node.clone(),
                    })?;

            state = node_fn(state).await?;
            steps += 1;

            // Resolve next node: conditional edges take priority.
            current_node =
                if let Some((condition, mapping)) = self.conditional_edges.get(&current_node) {
                    let result = condition(&state);
                    mapping.get(&result).cloned().unwrap_or(result)
                } else if let Some(targets) = self.edges.get(&current_node) {
                    targets
                        .first()
                        .cloned()
                        .ok_or_else(|| GraphError::CompileError {
                            message: format!("empty edge list for node '{current_node}'"),
                        })?
                } else {
                    return Err(GraphError::CompileError {
                        message: format!("no outgoing edge from node '{current_node}'"),
                    });
                };
        }

        Ok(state)
    }

    /// Generates a Mermaid diagram of the graph topology.
    ///
    /// Produces a `graph TD` (top-down) Mermaid diagram showing nodes
    /// and edges. This method is state-type-independent — it only
    /// inspects the graph topology, not the state type.
    pub fn to_mermaid(&self) -> String {
        let mut lines = vec!["graph TD".to_owned()];

        // Add entry edge.
        lines.push(format!(
            "    __start__([__start__]) --> {}",
            self.entry_point
        ));

        // Static edges.
        for (from, targets) in &self.edges {
            for to in targets {
                if to == END {
                    lines.push(format!("    {from} --> __end__([__end__])"));
                } else {
                    lines.push(format!("    {from} --> {to}"));
                }
            }
        }

        // Conditional edges.
        for (from, (_, mapping)) in &self.conditional_edges {
            for (label, to) in mapping {
                if to == END {
                    lines.push(format!("    {from} -->|{label}| __end__([__end__])"));
                } else {
                    lines.push(format!("    {from} -->|{label}| {to}"));
                }
            }
        }

        lines.join("\n")
    }
}

impl<S: State> fmt::Debug for CompiledGraph<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledGraph")
            .field("entry_point", &self.entry_point)
            .field("recursion_limit", &self.recursion_limit)
            .field("node_count", &self.nodes.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, unused_results)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use crate::constants::END;
    use crate::graph::{StateGraph, ValueState};

    /// T036: conditional edge with typed `ConditionFn<ValueState>` routes to correct branch.
    #[tokio::test]
    async fn t036_conditional_edge_routes_to_correct_branch() {
        // Case 1: state["go"] == true => route to "done"
        let mut graph = StateGraph::<ValueState>::new();
        graph
            .add_node("check", Box::new(|s| Box::pin(async move { Ok(s) })))
            .unwrap();
        graph
            .add_node(
                "done",
                Box::new(|mut s: ValueState| {
                    Box::pin(async move {
                        s.0["result"] = json!("completed");
                        Ok(s)
                    })
                }),
            )
            .unwrap();

        graph.set_entry_point("check");
        graph.add_conditional_edges(
            "check",
            Box::new(|s: &ValueState| {
                if s.0
                    .get("go")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    "done".to_owned()
                } else {
                    END.to_owned()
                }
            }),
            HashMap::new(),
        );
        graph.set_finish_point("done");

        let compiled = graph.compile().unwrap();

        // go == true => should route to "done"
        let result = compiled
            .invoke(ValueState(json!({"go": true})))
            .await
            .unwrap();
        assert_eq!(result.0["result"], "completed");

        // Case 2: go == false => route to END directly
        let mut graph2 = StateGraph::<ValueState>::new();
        graph2
            .add_node("check", Box::new(|s| Box::pin(async move { Ok(s) })))
            .unwrap();
        graph2
            .add_node(
                "done",
                Box::new(|mut s: ValueState| {
                    Box::pin(async move {
                        s.0["result"] = json!("completed");
                        Ok(s)
                    })
                }),
            )
            .unwrap();
        graph2.set_entry_point("check");
        graph2.add_conditional_edges(
            "check",
            Box::new(|s: &ValueState| {
                if s.0
                    .get("go")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    "done".to_owned()
                } else {
                    END.to_owned()
                }
            }),
            HashMap::new(),
        );
        graph2.set_finish_point("done");

        let compiled2 = graph2.compile().unwrap();
        let result2 = compiled2
            .invoke(ValueState(json!({"go": false})))
            .await
            .unwrap();
        assert!(result2.0.get("result").is_none());
    }

    /// T037: multi-branch conditional edge with 3+ branches on typed state.
    #[tokio::test]
    async fn t037_multi_branch_conditional_edge() {
        let mut graph = StateGraph::<ValueState>::new();
        graph
            .add_node("router", Box::new(|s| Box::pin(async move { Ok(s) })))
            .unwrap();
        graph
            .add_node(
                "a",
                Box::new(|mut s: ValueState| {
                    Box::pin(async move {
                        s.0["visited"] = json!("a");
                        Ok(s)
                    })
                }),
            )
            .unwrap();
        graph
            .add_node(
                "b",
                Box::new(|mut s: ValueState| {
                    Box::pin(async move {
                        s.0["visited"] = json!("b");
                        Ok(s)
                    })
                }),
            )
            .unwrap();
        graph
            .add_node(
                "c",
                Box::new(|mut s: ValueState| {
                    Box::pin(async move {
                        s.0["visited"] = json!("c");
                        Ok(s)
                    })
                }),
            )
            .unwrap();

        graph.set_entry_point("router");

        let mut mapping = HashMap::new();
        mapping.insert("a".to_owned(), "a".to_owned());
        mapping.insert("b".to_owned(), "b".to_owned());
        mapping.insert("c".to_owned(), "c".to_owned());

        graph.add_conditional_edges(
            "router",
            Box::new(|s: &ValueState| {
                s.0.get("branch")
                    .and_then(|v| v.as_str())
                    .unwrap_or("a")
                    .to_owned()
            }),
            mapping,
        );

        graph.set_finish_point("a");
        graph.set_finish_point("b");
        graph.set_finish_point("c");

        let compiled = graph.compile().unwrap();

        // Branch a
        let result = compiled
            .invoke(ValueState(json!({"branch": "a"})))
            .await
            .unwrap();
        assert_eq!(result.0["visited"], "a");

        // Branch b
        let result = compiled
            .invoke(ValueState(json!({"branch": "b"})))
            .await
            .unwrap();
        assert_eq!(result.0["visited"], "b");

        // Branch c
        let result = compiled
            .invoke(ValueState(json!({"branch": "c"})))
            .await
            .unwrap();
        assert_eq!(result.0["visited"], "c");
    }

    /// T038: looping graph with condition terminates after correct iterations.
    #[tokio::test]
    async fn t038_looping_graph_terminates_after_correct_iterations() {
        let mut graph = StateGraph::<ValueState>::new();
        graph
            .add_node(
                "increment",
                Box::new(|mut s: ValueState| {
                    Box::pin(async move {
                        let counter =
                            s.0.get("counter")
                                .and_then(serde_json::Value::as_i64)
                                .unwrap_or(0);
                        s.0["counter"] = json!(counter + 1);
                        Ok(s)
                    })
                }),
            )
            .unwrap();

        graph.set_entry_point("increment");
        graph.add_conditional_edges(
            "increment",
            Box::new(|s: &ValueState| {
                let counter =
                    s.0.get("counter")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0);
                if counter < 3 {
                    "increment".to_owned()
                } else {
                    END.to_owned()
                }
            }),
            HashMap::new(),
        );

        let compiled = graph.compile().unwrap();
        let result = compiled
            .invoke(ValueState(json!({"counter": 0})))
            .await
            .unwrap();
        assert_eq!(result.0["counter"], 3);
    }
}

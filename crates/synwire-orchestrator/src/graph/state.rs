//! State trait and graph builder.
//!
//! The [`State`] trait defines typed, serialisable graph state. Use
//! `#[derive(State)]` for automatic implementation.
//!
//! [`StateGraph`] is the primary entry point for defining a graph-based
//! state machine. Add nodes (async functions that transform state) and
//! edges (transitions), then compile into a [`CompiledGraph`] for execution.

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::channels::BaseChannel;
use crate::constants::{END, START};
use crate::error::GraphError;
use crate::graph::compiled::CompiledGraph;

/// Trait for typed graph state.
///
/// Implementors define the channel configuration for their fields and
/// provide deserialisation from channel values. Use `#[derive(State)]`
/// for automatic implementation.
///
/// # Example
///
/// ```rust,ignore
/// use synwire_orchestrator::graph::State;
/// use synwire_derive::State;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize, State)]
/// struct MyState {
///     counter: i32,
///     #[reducer(topic)]
///     messages: Vec<String>,
/// }
/// ```
pub trait State: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {
    /// Returns channel configuration for each field in this state.
    fn channels() -> Vec<(String, Box<dyn BaseChannel>)>;

    /// Reconstructs this state from channel values.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DeserializationError`] if a channel value
    /// cannot be deserialised into the expected field type.
    fn from_channels(channels: &HashMap<String, Box<dyn BaseChannel>>) -> Result<Self, GraphError>;

    /// Serialises this state to a JSON Value for checkpoint storage.
    ///
    /// Default implementation uses `serde_json::to_value`.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::Checkpoint`] if serialisation fails.
    fn to_value(&self) -> Result<serde_json::Value, GraphError> {
        serde_json::to_value(self).map_err(|e| GraphError::Checkpoint {
            message: format!("failed to serialise state: {e}"),
        })
    }

    /// Deserialises a state from a JSON Value (checkpoint restore).
    ///
    /// Default implementation uses `serde_json::from_value`.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::Checkpoint`] if deserialisation fails.
    fn from_value(value: serde_json::Value) -> Result<Self, GraphError> {
        serde_json::from_value(value).map_err(|e| GraphError::Checkpoint {
            message: format!("failed to deserialise state: {e}"),
        })
    }
}

/// A boxed async node function operating on typed state.
///
/// Accepts the current state `S` and returns the updated state (or an error).
pub type NodeFn<S> =
    Box<dyn Fn(S) -> synwire_core::BoxFuture<'static, Result<S, GraphError>> + Send + Sync>;

/// A boxed condition function for conditional edges.
///
/// Inspects the current state and returns the name of the branch to follow.
pub type ConditionFn<S> = Box<dyn Fn(&S) -> String + Send + Sync>;

/// A builder for constructing typed state graphs.
///
/// # Example
///
/// ```rust,ignore
/// use synwire_orchestrator::graph::{StateGraph, ValueState};
/// use synwire_orchestrator::constants::END;
///
/// let mut graph = StateGraph::<ValueState>::new();
/// graph
///     .add_node("greet", Box::new(|state| {
///         Box::pin(async move { Ok(state) })
///     }))?
///     .set_entry_point("greet")
///     .add_edge("greet", END);
/// let compiled = graph.compile()?;
/// ```
pub struct StateGraph<S: State> {
    nodes: HashMap<String, NodeFn<S>>,
    edges: Vec<(String, String)>,
    conditional_edges: Vec<(String, ConditionFn<S>, HashMap<String, String>)>,
    entry_point: Option<String>,
    finish_points: Vec<String>,
}

impl<S: State> StateGraph<S> {
    /// Creates a new empty state graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            conditional_edges: Vec::new(),
            entry_point: None,
            finish_points: Vec::new(),
        }
    }

    /// Adds a node with the given name and function.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DuplicateNode`] if a node with the same name
    /// already exists.
    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        func: NodeFn<S>,
    ) -> Result<&mut Self, GraphError> {
        let name = name.into();
        if self.nodes.contains_key(&name) {
            return Err(GraphError::DuplicateNode { name });
        }
        let _prev = self.nodes.insert(name, func);
        Ok(self)
    }

    /// Adds a static edge from one node to another.
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        self.edges.push((from.into(), to.into()));
        self
    }

    /// Adds a conditional edge from a node.
    ///
    /// The `condition` function inspects the typed state and returns a key.
    /// That key is looked up in `mapping` to determine the actual target node.
    /// If the key is not found in the mapping, it is used as the target
    /// node name directly.
    pub fn add_conditional_edges(
        &mut self,
        from: impl Into<String>,
        condition: ConditionFn<S>,
        mapping: HashMap<String, String>,
    ) -> &mut Self {
        self.conditional_edges
            .push((from.into(), condition, mapping));
        self
    }

    /// Sets the entry point of the graph.
    ///
    /// This is equivalent to adding an edge from [`START`] to the given node.
    pub fn set_entry_point(&mut self, name: impl Into<String>) -> &mut Self {
        self.entry_point = Some(name.into());
        self
    }

    /// Sets a finish point for the graph.
    ///
    /// This is equivalent to adding an edge from the given node to [`END`].
    pub fn set_finish_point(&mut self, name: impl Into<String>) -> &mut Self {
        self.finish_points.push(name.into());
        self
    }

    /// Compiles the graph into a [`CompiledGraph`] ready for execution.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::NoEntryPoint`] if no entry point was set.
    /// Returns [`GraphError::CompileError`] if the graph topology is invalid.
    pub fn compile(self) -> Result<CompiledGraph<S>, GraphError> {
        let entry_point = self.entry_point.ok_or(GraphError::NoEntryPoint)?;

        // Validate that the entry point references a known node.
        if !self.nodes.contains_key(&entry_point) {
            return Err(GraphError::CompileError {
                message: format!("entry point '{entry_point}' is not a known node"),
            });
        }

        // Build the edge map.
        let mut edge_map: HashMap<String, Vec<String>> = HashMap::new();
        for (from, to) in &self.edges {
            edge_map.entry(from.clone()).or_default().push(to.clone());
        }

        // Add finish points as edges to END.
        for fp in &self.finish_points {
            edge_map.entry(fp.clone()).or_default().push(END.to_owned());
        }

        // Build conditional edge map.
        let mut cond_edge_map: HashMap<String, (ConditionFn<S>, HashMap<String, String>)> =
            HashMap::new();
        for (from, condition, mapping) in self.conditional_edges {
            let _prev = cond_edge_map.insert(from, (condition, mapping));
        }

        // Validate: every non-END, non-conditional node must have at least one outgoing edge.
        for name in self.nodes.keys() {
            if !edge_map.contains_key(name) && !cond_edge_map.contains_key(name) {
                return Err(GraphError::CompileError {
                    message: format!("node '{name}' has no outgoing edges"),
                });
            }
        }

        // Validate: edge targets reference known nodes or END/START.
        for targets in edge_map.values() {
            for target in targets {
                if target != END && target != START && !self.nodes.contains_key(target) {
                    return Err(GraphError::CompileError {
                        message: format!("edge target '{target}' is not a known node"),
                    });
                }
            }
        }

        Ok(CompiledGraph::new(
            self.nodes,
            edge_map,
            cond_edge_map,
            entry_point,
        ))
    }
}

impl<S: State> Default for StateGraph<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use crate::graph::ValueState;

    use super::State;

    /// T042: `State::to_value()` serialises and `State::from_value()` deserialises round-trip.
    #[test]
    fn t042_to_value_from_value_round_trip() {
        let original = ValueState(json!({"name": "test", "count": 42, "active": true}));
        let serialised = original.to_value().unwrap();
        let restored = ValueState::from_value(serialised).unwrap();
        assert_eq!(original.0, restored.0);
    }

    /// T043: `to_value`/`from_value` round-trip with nested structures.
    #[test]
    fn t043_round_trip_nested_structures() {
        let original = ValueState(json!({
            "user": {
                "name": "Alice",
                "address": {
                    "city": "London",
                    "postcode": "SW1A 1AA"
                }
            },
            "tags": ["rust", "async", "graph"],
            "metadata": {
                "scores": [1, 2, 3],
                "nested": {
                    "deeply": {
                        "value": true
                    }
                }
            }
        }));
        let serialised = original.to_value().unwrap();
        let restored = ValueState::from_value(serialised).unwrap();
        assert_eq!(original.0, restored.0);
    }

    /// T044: `from_value` with `Value::Null` returns `ValueState(Null)`.
    #[test]
    fn t044_from_value_null_succeeds() {
        let restored = ValueState::from_value(serde_json::Value::Null).unwrap();
        assert!(restored.0.is_null());
    }

    /// T045: `to_value` and `from_value` with edge cases (empty object, null, arrays, deep nesting).
    #[test]
    fn t045_to_value_from_value_edge_cases() {
        // Empty object
        let state = ValueState(json!({}));
        let round_tripped = ValueState::from_value(state.to_value().unwrap()).unwrap();
        assert_eq!(round_tripped.0, json!({}));

        // Null
        let state = ValueState(serde_json::Value::Null);
        let round_tripped = ValueState::from_value(state.to_value().unwrap()).unwrap();
        assert!(round_tripped.0.is_null());

        // Array at top level
        let state = ValueState(json!([1, 2, 3, "four", null]));
        let round_tripped = ValueState::from_value(state.to_value().unwrap()).unwrap();
        assert_eq!(round_tripped.0, json!([1, 2, 3, "four", null]));

        // Deeply nested structure
        let state = ValueState(json!({
            "a": {"b": {"c": {"d": {"e": {"f": 42}}}}}
        }));
        let round_tripped = ValueState::from_value(state.to_value().unwrap()).unwrap();
        assert_eq!(round_tripped.0["a"]["b"]["c"]["d"]["e"]["f"], 42);

        // Large numeric values
        let state = ValueState(json!({"big": 9_007_199_254_740_992_i64, "negative": -999}));
        let round_tripped = ValueState::from_value(state.to_value().unwrap()).unwrap();
        assert_eq!(round_tripped.0, state.0);

        // Boolean and string edge cases
        let state = ValueState(json!({"empty_string": "", "unicode": "\u{1F600}", "bool": false}));
        let round_tripped = ValueState::from_value(state.to_value().unwrap()).unwrap();
        assert_eq!(round_tripped.0, state.0);
    }
}

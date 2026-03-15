//! Integration tests for `StateGraph` and `CompiledGraph`.

#![allow(clippy::unwrap_used, unused_results)]

use std::collections::HashMap;

use synwire_orchestrator::constants::END;
use synwire_orchestrator::graph::{NodeFn, StateGraph, ValueState};

fn passthrough_node() -> NodeFn<ValueState> {
    Box::new(|state| Box::pin(async move { Ok(state) }))
}

fn increment_node(field: &'static str) -> NodeFn<ValueState> {
    Box::new(move |mut state: ValueState| {
        Box::pin(async move {
            if let Some(obj) = state.0.as_object_mut() {
                let current = obj
                    .get(field)
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                obj.insert(field.to_owned(), serde_json::json!(current + 1));
            }
            Ok(state)
        })
    })
}

fn append_node(field: &'static str, value: &'static str) -> NodeFn<ValueState> {
    Box::new(move |mut state: ValueState| {
        Box::pin(async move {
            if let Some(obj) = state.0.as_object_mut() {
                let current = obj
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                obj.insert(
                    field.to_owned(),
                    serde_json::json!(format!("{current}{value}")),
                );
            }
            Ok(state)
        })
    })
}

// --- Compile tests ---

#[test]
fn compile_no_entry_point_fails() {
    let graph = StateGraph::<ValueState>::new();
    let result = graph.compile();
    assert!(result.is_err());
}

#[test]
fn compile_unknown_entry_point_fails() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.set_entry_point("nonexistent");
    let result = graph.compile();
    assert!(result.is_err());
}

#[test]
fn compile_node_without_edges_fails() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", passthrough_node()).unwrap();
    graph.set_entry_point("a");
    let result = graph.compile();
    assert!(result.is_err());
}

#[test]
fn compile_duplicate_node_fails() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", passthrough_node()).unwrap();
    let result = graph.add_node("a", passthrough_node());
    assert!(result.is_err());
}

#[test]
fn compile_valid_graph() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", passthrough_node()).unwrap();
    graph.set_entry_point("a").add_edge("a", END);
    let compiled = graph.compile().unwrap();
    assert_eq!(compiled.entry_point(), "a");
    assert_eq!(compiled.recursion_limit(), 25);
}

#[test]
fn compile_with_finish_point() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", passthrough_node()).unwrap();
    graph.set_entry_point("a").set_finish_point("a");
    let compiled = graph.compile().unwrap();
    assert_eq!(compiled.entry_point(), "a");
}

#[test]
fn compile_edge_to_unknown_node_fails() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", passthrough_node()).unwrap();
    graph.set_entry_point("a").add_edge("a", "nonexistent");
    let result = graph.compile();
    assert!(result.is_err());
}

// --- Invoke tests ---

#[tokio::test]
async fn invoke_single_node() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("echo", passthrough_node()).unwrap();
    graph.set_entry_point("echo").add_edge("echo", END);

    let compiled = graph.compile().unwrap();
    let result = compiled
        .invoke(ValueState(serde_json::json!({"msg": "hi"})))
        .await
        .unwrap();
    assert_eq!(result.0, serde_json::json!({"msg": "hi"}));
}

#[tokio::test]
async fn invoke_three_node_linear() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", append_node("path", "a")).unwrap();
    graph.add_node("b", append_node("path", "b")).unwrap();
    graph.add_node("c", append_node("path", "c")).unwrap();
    graph
        .set_entry_point("a")
        .add_edge("a", "b")
        .add_edge("b", "c")
        .add_edge("c", END);

    let compiled = graph.compile().unwrap();
    let result = compiled
        .invoke(ValueState(serde_json::json!({"path": ""})))
        .await
        .unwrap();
    assert_eq!(result.0["path"], "abc");
}

#[tokio::test]
async fn invoke_conditional_edges() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("router", passthrough_node()).unwrap();
    graph
        .add_node("left", append_node("result", "left"))
        .unwrap();
    graph
        .add_node("right", append_node("result", "right"))
        .unwrap();

    let mut mapping = HashMap::new();
    mapping.insert("go_left".to_owned(), "left".to_owned());
    mapping.insert("go_right".to_owned(), "right".to_owned());

    graph
        .set_entry_point("router")
        .add_conditional_edges(
            "router",
            Box::new(|state: &ValueState| {
                state
                    .0
                    .get("direction")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("go_left")
                    .to_owned()
            }),
            mapping,
        )
        .add_edge("left", END)
        .add_edge("right", END);

    let compiled = graph.compile().unwrap();

    // Go left.
    let result = compiled
        .invoke(ValueState(
            serde_json::json!({"direction": "go_left", "result": ""}),
        ))
        .await
        .unwrap();
    assert_eq!(result.0["result"], "left");

    // Go right.
    let result = compiled
        .invoke(ValueState(
            serde_json::json!({"direction": "go_right", "result": ""}),
        ))
        .await
        .unwrap();
    assert_eq!(result.0["result"], "right");
}

#[tokio::test]
async fn invoke_recursion_limit() {
    let mut graph = StateGraph::<ValueState>::new();
    graph
        .add_node("loop_node", increment_node("count"))
        .unwrap();
    // Self-loop: never reaches END.
    graph
        .set_entry_point("loop_node")
        .add_edge("loop_node", "loop_node");

    let compiled = graph.compile().unwrap().with_recursion_limit(5);
    let result = compiled
        .invoke(ValueState(serde_json::json!({"count": 0})))
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("recursion limit"), "got: {msg}");
}

#[tokio::test]
async fn invoke_conditional_to_end() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("check", passthrough_node()).unwrap();

    let mut mapping = HashMap::new();
    mapping.insert("done".to_owned(), END.to_owned());
    mapping.insert("continue".to_owned(), "check".to_owned());

    graph.set_entry_point("check").add_conditional_edges(
        "check",
        Box::new(|state: &ValueState| {
            if state
                .0
                .get("done")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                "done".to_owned()
            } else {
                "continue".to_owned()
            }
        }),
        mapping,
    );

    let compiled = graph.compile().unwrap().with_recursion_limit(3);

    // Already done: should exit immediately after one step.
    let result = compiled
        .invoke(ValueState(serde_json::json!({"done": true})))
        .await
        .unwrap();
    assert_eq!(result.0["done"], true);
}

// --- Mermaid tests ---

#[test]
fn mermaid_output_contains_nodes_and_edges() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", passthrough_node()).unwrap();
    graph.add_node("b", passthrough_node()).unwrap();
    graph
        .set_entry_point("a")
        .add_edge("a", "b")
        .add_edge("b", END);

    let compiled = graph.compile().unwrap();
    let mermaid = compiled.to_mermaid();

    assert!(mermaid.contains("graph TD"), "missing graph TD header");
    assert!(mermaid.contains("__start__"), "missing start node");
    assert!(mermaid.contains("__end__"), "missing end node");
    assert!(mermaid.contains('a'), "missing node a");
    assert!(mermaid.contains('b'), "missing node b");
}

// --- Debug impl ---

#[test]
fn debug_format() {
    let mut graph = StateGraph::<ValueState>::new();
    graph.add_node("a", passthrough_node()).unwrap();
    graph.set_entry_point("a").add_edge("a", END);
    let compiled = graph.compile().unwrap();
    let debug = format!("{compiled:?}");
    assert!(debug.contains("CompiledGraph"));
    assert!(debug.contains("entry_point"));
}

// --- sync_node helper ---

#[tokio::test]
async fn sync_node_helper() {
    use synwire_orchestrator::func::sync_node;

    let mut graph = StateGraph::<ValueState>::new();
    graph
        .add_node(
            "double",
            sync_node(|mut state: ValueState| {
                if let Some(n) = state.0.get("n").and_then(serde_json::Value::as_i64) {
                    if let Some(obj) = state.0.as_object_mut() {
                        obj.insert("n".to_owned(), serde_json::json!(n * 2));
                    }
                }
                Ok(state)
            }),
        )
        .unwrap();
    graph.set_entry_point("double").add_edge("double", END);

    let compiled = graph.compile().unwrap();
    let result = compiled
        .invoke(ValueState(serde_json::json!({"n": 21})))
        .await
        .unwrap();
    assert_eq!(result.0["n"], 42);
}

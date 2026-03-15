//! Integration tests for the `#[tool]` attribute macro.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unused_async)]

use synwire_core::error::SynwireError;
use synwire_core::tools::Tool;
use synwire_derive::tool;

#[tool]
/// Searches the web for information.
async fn search(query: String) -> Result<String, SynwireError> {
    Ok(format!("Results for: {query}"))
}

#[tool]
/// Adds two integers.
async fn add(a: i64, b: i64) -> Result<String, SynwireError> {
    Ok(format!("{}", a + b))
}

#[tool]
async fn no_doc_tool(input: String) -> Result<String, SynwireError> {
    Ok(input)
}

#[tokio::test]
async fn tool_macro_builds_structured_tool() {
    let tool = search_tool().expect("tool should build");
    assert_eq!(tool.name(), "search");
    assert_eq!(tool.description(), "Searches the web for information.");
}

#[tokio::test]
async fn tool_macro_invokes_correctly() {
    let tool = search_tool().expect("tool should build");
    let result = tool
        .invoke(serde_json::json!({"query": "rust lang"}))
        .await
        .expect("invoke should succeed");
    assert_eq!(result.content, "Results for: rust lang");
    assert!(result.artifact.is_none());
}

#[tokio::test]
async fn tool_macro_integer_params() {
    let tool = add_tool().expect("tool should build");
    let result = tool
        .invoke(serde_json::json!({"a": 3, "b": 4}))
        .await
        .expect("invoke should succeed");
    assert_eq!(result.content, "7");
}

#[tokio::test]
async fn tool_macro_schema_has_params() {
    let tool = search_tool().expect("tool should build");
    let schema = tool.schema();
    let params = &schema.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["query"].is_object());
    assert_eq!(params["properties"]["query"]["type"], "string");
}

#[tokio::test]
async fn tool_macro_no_doc_uses_fn_name() {
    let tool = no_doc_tool_tool().expect("tool should build");
    assert_eq!(tool.description(), "no_doc_tool");
}

#[tokio::test]
async fn tool_macro_schema_required_fields() {
    let tool = add_tool().expect("tool should build");
    let required = tool.schema().parameters["required"]
        .as_array()
        .expect("required should be array");
    assert_eq!(required.len(), 2);
    assert!(required.contains(&serde_json::json!("a")));
    assert!(required.contains(&serde_json::json!("b")));
}

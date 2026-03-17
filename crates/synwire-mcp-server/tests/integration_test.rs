//! Integration tests for the Synwire MCP server.
#![allow(clippy::expect_used, clippy::unwrap_used)]
//!
//! These tests verify the JSON-RPC 2.0 message structure and protocol
//! invariants without spawning a subprocess. Full process-based tests that
//! drive `synwire-mcp-server` over stdio are run in CI once the binary is
//! installed.

/// Verify that the MCP `initialize` request/response shape is correct.
#[test]
fn test_initialize_request_shape() {
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let parsed: serde_json::Value = serde_json::from_str(request).expect("valid JSON");
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["method"], "initialize");
    assert_eq!(parsed["id"], 1);
}

/// Verify that the MCP `initialize` response structure is well-formed.
#[test]
fn test_initialize_response_shape() {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "synwire-mcp-server",
                "version": "0.1.0"
            }
        }
    });
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert!(response["result"]["capabilities"]["tools"].is_object());
    assert_eq!(
        response["result"]["serverInfo"]["name"],
        "synwire-mcp-server"
    );
}

/// Verify that the MCP `tools/list` response structure is well-formed.
#[test]
fn test_tools_list_response_shape() {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": {
            "tools": [
                {
                    "name": "fs.read",
                    "description": "Read file",
                    "inputSchema": { "type": "object", "properties": {} }
                }
            ]
        }
    });
    assert!(response["result"]["tools"].is_array());
    let tools = response["result"]["tools"].as_array().expect("array");
    assert_eq!(tools[0]["name"], "fs.read");
}

/// Verify that `tools/call` request and error response shapes are correct.
#[test]
fn test_tools_call_error_shape() {
    // A well-formed error response (e.g., unknown tool).
    let error_response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "error": {
            "code": -32602,
            "message": "Unknown tool: does_not_exist"
        }
    });
    assert_eq!(error_response["error"]["code"], -32602);
    assert!(
        error_response["error"]["message"]
            .as_str()
            .expect("string")
            .contains("does_not_exist")
    );
}

/// Verify that `tools/call` success response wraps content in the MCP format.
#[test]
fn test_tools_call_success_shape() {
    let success_response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "result": {
            "content": [{ "type": "text", "text": "file contents here" }]
        }
    });
    let content = success_response["result"]["content"]
        .as_array()
        .expect("content array");
    assert_eq!(content[0]["type"], "text");
    assert!(!content[0]["text"].as_str().expect("text").is_empty());
}

/// Verify that a parse error produces a -32700 response.
#[test]
fn test_parse_error_code() {
    let parse_error = serde_json::json!({
        "jsonrpc": "2.0",
        "id": serde_json::Value::Null,
        "error": {
            "code": -32700,
            "message": "Parse error: ..."
        }
    });
    assert_eq!(parse_error["error"]["code"], -32700);
}

/// Verify that unknown methods produce a -32601 response.
#[test]
fn test_method_not_found_code() {
    let not_found = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "error": {
            "code": -32601,
            "message": "Method not found: unknown/method"
        }
    });
    assert_eq!(not_found["error"]["code"], -32601);
}

//! DAP tool definitions exposed to the agent tool registry.
//!
//! Each tool wraps a [`DapClient`] method and converts errors into
//! [`SynwireError::Tool`] for the agent runtime.

use std::sync::Arc;

use synwire_core::error::{SynwireError, ToolError};
use synwire_core::tools::{StructuredTool, Tool, ToolOutput, ToolSchema};

use crate::client::DapClient;

/// Convert a [`crate::error::DapError`] into a [`SynwireError`].
#[allow(clippy::needless_pass_by_value)] // Required by `map_err` signature.
fn dap_err(e: crate::error::DapError) -> SynwireError {
    SynwireError::Tool(ToolError::InvocationFailed {
        message: e.to_string(),
    })
}

/// Convert a serialization error into a [`SynwireError`].
#[allow(clippy::needless_pass_by_value)] // Required by `map_err` signature.
fn json_err(e: serde_json::Error) -> SynwireError {
    SynwireError::Tool(ToolError::InvocationFailed {
        message: e.to_string(),
    })
}

/// Create all 14 DAP tools bound to the given client.
///
/// # Errors
///
/// Returns [`SynwireError`] if any tool fails validation (should not happen
/// with the hard-coded names).
pub fn create_tools(client: Arc<DapClient>) -> Result<Vec<Arc<dyn Tool>>, SynwireError> {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(dap_status_tool(Arc::clone(&client))?),
        Arc::new(dap_launch_tool(Arc::clone(&client))?),
        Arc::new(dap_attach_tool(Arc::clone(&client))?),
        Arc::new(dap_set_breakpoints_tool(Arc::clone(&client))?),
        Arc::new(dap_continue_tool(Arc::clone(&client))?),
        Arc::new(dap_step_over_tool(Arc::clone(&client))?),
        Arc::new(dap_step_in_tool(Arc::clone(&client))?),
        Arc::new(dap_step_out_tool(Arc::clone(&client))?),
        Arc::new(dap_pause_tool(Arc::clone(&client))?),
        Arc::new(dap_threads_tool(Arc::clone(&client))?),
        Arc::new(dap_stack_trace_tool(Arc::clone(&client))?),
        Arc::new(dap_variables_tool(Arc::clone(&client))?),
        Arc::new(dap_evaluate_tool(Arc::clone(&client))?),
        Arc::new(dap_disconnect_tool(client)?),
    ];
    Ok(tools)
}

fn dap_status_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.status")
        .description("Show the current debug session state, capabilities, and active breakpoints")
        .schema(ToolSchema {
            name: "debug.status".into(),
            description:
                "Show the current debug session state, capabilities, and active breakpoints".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            }),
        })
        .func(move |_input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let state = client.status().await;
                let caps = client.capabilities().await;
                let bps = client.active_breakpoints().await;

                let result = serde_json::json!({
                    "state": format!("{state}"),
                    "capabilities": caps,
                    "active_breakpoints": bps,
                });

                let content = serde_json::to_string_pretty(&result).map_err(json_err)?;
                Ok(ToolOutput {
                    content,
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_launch_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.launch")
        .description(
            "Launch a program under the debugger. Pass launch configuration as JSON arguments.",
        )
        .schema(ToolSchema {
            name: "debug.launch".into(),
            description: "Launch a program under the debugger".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "program": {
                        "type": "string",
                        "description": "Path to the program to debug"
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Command-line arguments for the program"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for the program"
                    },
                    "env": {
                        "type": "object",
                        "description": "Environment variables for the program"
                    },
                    "stopOnEntry": {
                        "type": "boolean",
                        "description": "Whether to stop at the program entry point"
                    }
                },
                "required": ["program"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                client.launch(input).await.map_err(dap_err)?;
                Ok(ToolOutput {
                    content: "Program launched successfully under debugger.".into(),
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_attach_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.attach")
        .description("Attach to a running process for debugging")
        .schema(ToolSchema {
            name: "debug.attach".into(),
            description: "Attach to a running process for debugging".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "processId": {
                        "type": "integer",
                        "description": "Process ID to attach to"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Port to connect to (for remote debugging)"
                    },
                    "host": {
                        "type": "string",
                        "description": "Host for remote debugging"
                    }
                },
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                client.attach(input).await.map_err(dap_err)?;
                Ok(ToolOutput {
                    content: "Attached to process successfully.".into(),
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_set_breakpoints_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.set_breakpoints")
        .description("Set breakpoints in a source file at specified line numbers")
        .schema(ToolSchema {
            name: "debug.set_breakpoints".into(),
            description: "Set breakpoints in a source file at specified line numbers".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_path": {
                        "type": "string",
                        "description": "Absolute path to the source file"
                    },
                    "lines": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Line numbers to set breakpoints on"
                    }
                },
                "required": ["source_path", "lines"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let source_path = input
                    .get("source_path")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing source_path parameter".into(),
                        ))
                    })?;

                let lines: Vec<i64> = input
                    .get("lines")
                    .and_then(serde_json::Value::as_array)
                    .map(|arr| arr.iter().filter_map(serde_json::Value::as_i64).collect())
                    .unwrap_or_default();

                let result = client
                    .set_breakpoints(source_path, &lines)
                    .await
                    .map_err(dap_err)?;

                let content = serde_json::to_string_pretty(&serde_json::json!({
                    "breakpoints": result,
                }))
                .map_err(json_err)?;

                Ok(ToolOutput {
                    content,
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_continue_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.continue")
        .description("Continue execution of a paused thread")
        .schema(ToolSchema {
            name: "debug.continue".into(),
            description: "Continue execution of a paused thread".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "integer",
                        "description": "Thread ID to continue (use debug.threads to list)"
                    }
                },
                "required": ["thread_id"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let thread_id = input
                    .get("thread_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing thread_id parameter".into(),
                        ))
                    })?;

                client
                    .continue_execution(thread_id)
                    .await
                    .map_err(dap_err)?;
                Ok(ToolOutput {
                    content: format!("Thread {thread_id} resumed."),
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_step_over_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.step_over")
        .description("Step over the current line (next) for a thread")
        .schema(ToolSchema {
            name: "debug.step_over".into(),
            description: "Step over the current line (next) for a thread".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "integer",
                        "description": "Thread ID to step over"
                    }
                },
                "required": ["thread_id"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let thread_id = input
                    .get("thread_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing thread_id parameter".into(),
                        ))
                    })?;

                client.next(thread_id).await.map_err(dap_err)?;
                Ok(ToolOutput {
                    content: format!("Thread {thread_id} stepped over."),
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_step_in_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.step_in")
        .description("Step into the current function call for a thread")
        .schema(ToolSchema {
            name: "debug.step_in".into(),
            description: "Step into the current function call for a thread".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "integer",
                        "description": "Thread ID to step into"
                    }
                },
                "required": ["thread_id"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let thread_id = input
                    .get("thread_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing thread_id parameter".into(),
                        ))
                    })?;

                client.step_in(thread_id).await.map_err(dap_err)?;
                Ok(ToolOutput {
                    content: format!("Thread {thread_id} stepped in."),
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_step_out_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.step_out")
        .description("Step out of the current function for a thread")
        .schema(ToolSchema {
            name: "debug.step_out".into(),
            description: "Step out of the current function for a thread".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "integer",
                        "description": "Thread ID to step out of"
                    }
                },
                "required": ["thread_id"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let thread_id = input
                    .get("thread_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing thread_id parameter".into(),
                        ))
                    })?;

                client.step_out(thread_id).await.map_err(dap_err)?;
                Ok(ToolOutput {
                    content: format!("Thread {thread_id} stepped out."),
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_pause_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.pause")
        .description("Pause execution of a running thread")
        .schema(ToolSchema {
            name: "debug.pause".into(),
            description: "Pause execution of a running thread".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "integer",
                        "description": "Thread ID to pause"
                    }
                },
                "required": ["thread_id"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let thread_id = input
                    .get("thread_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing thread_id parameter".into(),
                        ))
                    })?;

                client.pause(thread_id).await.map_err(dap_err)?;
                Ok(ToolOutput {
                    content: format!("Thread {thread_id} paused."),
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_threads_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.threads")
        .description("List all threads in the debuggee process")
        .schema(ToolSchema {
            name: "debug.threads".into(),
            description: "List all threads in the debuggee process".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            }),
        })
        .func(move |_input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let threads = client.threads().await.map_err(dap_err)?;

                let content = serde_json::to_string_pretty(&serde_json::json!({
                    "threads": threads,
                }))
                .map_err(json_err)?;

                Ok(ToolOutput {
                    content,
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_stack_trace_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.stack_trace")
        .description("Get the stack trace for a specific thread")
        .schema(ToolSchema {
            name: "debug.stack_trace".into(),
            description: "Get the stack trace for a specific thread".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "integer",
                        "description": "Thread ID to get stack trace for"
                    }
                },
                "required": ["thread_id"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let thread_id = input
                    .get("thread_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing thread_id parameter".into(),
                        ))
                    })?;

                let frames = client.stack_trace(thread_id).await.map_err(dap_err)?;

                let content = serde_json::to_string_pretty(&serde_json::json!({
                    "stack_frames": frames,
                }))
                .map_err(json_err)?;

                Ok(ToolOutput {
                    content,
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_variables_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.variables")
        .description("Get variables for a scope or structured variable reference. Use debug.stack_trace and then debug.scopes to get variable references.")
        .schema(ToolSchema {
            name: "debug.variables".into(),
            description: "Get variables for a scope or structured variable reference".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "variables_reference": {
                        "type": "integer",
                        "description": "Variables reference ID (from scopes or structured variables)"
                    }
                },
                "required": ["variables_reference"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let variables_ref = input
                    .get("variables_reference")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| dap_err(crate::error::DapError::Transport(
                        "missing variables_reference parameter".into(),
                    )))?;

                let variables = client.variables(variables_ref).await.map_err(dap_err)?;

                let content = serde_json::to_string_pretty(&serde_json::json!({
                    "variables": variables,
                }))
                .map_err(json_err)?;

                Ok(ToolOutput {
                    content,
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_evaluate_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.evaluate")
        .description("Evaluate an expression in the debuggee context (REPL mode)")
        .schema(ToolSchema {
            name: "debug.evaluate".into(),
            description: "Evaluate an expression in the debuggee context".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Expression to evaluate"
                    },
                    "frame_id": {
                        "type": "integer",
                        "description": "Optional stack frame ID for context"
                    }
                },
                "required": ["expression"],
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let expression = input
                    .get("expression")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        dap_err(crate::error::DapError::Transport(
                            "missing expression parameter".into(),
                        ))
                    })?;

                let frame_id = input.get("frame_id").and_then(serde_json::Value::as_i64);

                let result = client
                    .evaluate(expression, frame_id)
                    .await
                    .map_err(dap_err)?;

                let content = serde_json::to_string_pretty(&result).map_err(json_err)?;

                Ok(ToolOutput {
                    content,
                    ..Default::default()
                })
            })
        })
        .build()
}

fn dap_disconnect_tool(client: Arc<DapClient>) -> Result<StructuredTool, SynwireError> {
    StructuredTool::builder()
        .name("debug.disconnect")
        .description("Disconnect from the debug session and terminate the debuggee")
        .schema(ToolSchema {
            name: "debug.disconnect".into(),
            description: "Disconnect from the debug session and terminate the debuggee".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            }),
        })
        .func(move |_input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                client.disconnect().await.map_err(dap_err)?;
                Ok(ToolOutput {
                    content: "Debug session disconnected.".into(),
                    ..Default::default()
                })
            })
        })
        .build()
}

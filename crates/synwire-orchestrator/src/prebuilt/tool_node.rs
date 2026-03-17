//! Tool execution node.
//!
//! [`ToolNode`] inspects the state for tool calls (from an AI message) and
//! executes them sequentially, appending tool-response messages to the state.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use synwire_core::tools::{TimeoutBehavior, Tool, ToolConfig};

use crate::error::GraphError;

/// A single tool execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultEntry {
    /// The tool call ID this result corresponds to.
    pub tool_call_id: String,
    /// The tool name.
    pub tool_name: String,
    /// The result content (may be truncated).
    pub content: String,
    /// Whether the result was truncated to fit size limits.
    pub truncated: bool,
}

/// Node that executes tool calls from the state.
///
/// Reads `tool_calls` from the last AI message in the state's `"messages"`
/// array, invokes each tool sequentially, and appends tool-response messages
/// back into the messages list.
///
/// # State contract
///
/// **Input:** State must contain a `"messages"` array where the last entry is
/// an AI message with a `"tool_calls"` array.
///
/// **Output:** The same state with tool-response messages appended.
pub struct ToolNode {
    tools: HashMap<String, Box<dyn Tool>>,
    max_result_size: Option<usize>,
    /// Per-tool operational configuration keyed by tool name.
    tool_configs: HashMap<String, ToolConfig>,
    /// Default timeout applied to all tools without a per-tool override.
    default_timeout: Option<Duration>,
}

impl ToolNode {
    /// Creates a new `ToolNode` from a list of tools.
    ///
    /// Tools are indexed by their [`Tool::name`] for O(1) lookup.
    pub fn new(tools: Vec<Box<dyn Tool>>) -> Self {
        let tools = tools
            .into_iter()
            .map(|t| (t.name().to_owned(), t))
            .collect();
        Self {
            tools,
            max_result_size: None,
            tool_configs: HashMap::new(),
            default_timeout: None,
        }
    }

    /// Sets the maximum result size in bytes.
    ///
    /// Results exceeding this limit are truncated and the `truncated` flag
    /// is set on the [`ToolResultEntry`].
    #[must_use]
    pub const fn with_max_result_size(mut self, size: usize) -> Self {
        self.max_result_size = Some(size);
        self
    }

    /// Sets per-tool operational configuration.
    ///
    /// The `name` must match the tool's [`Tool::name`]. Per-tool timeouts
    /// take precedence over the default timeout set with
    /// [`with_default_timeout`](Self::with_default_timeout).
    #[must_use]
    pub fn with_tool_config(mut self, name: impl Into<String>, config: ToolConfig) -> Self {
        let _ = self.tool_configs.insert(name.into(), config);
        self
    }

    /// Sets the default timeout applied to every tool invocation.
    ///
    /// Per-tool configurations set via [`with_tool_config`](Self::with_tool_config)
    /// override this value when they specify their own timeout.
    #[must_use]
    pub const fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = Some(timeout);
        self
    }

    /// Executes tool calls found in the state.
    ///
    /// # Errors
    ///
    /// - [`GraphError::InvalidUpdate`] if state has no `"messages"` array or
    ///   the last message has no tool calls.
    /// - [`GraphError::ToolNotFound`] if a tool call references an unknown tool.
    /// - [`GraphError::ToolInvocation`] if a tool returns an error.
    #[allow(clippy::too_many_lines)]
    pub async fn invoke(
        &self,
        mut state: serde_json::Value,
    ) -> Result<serde_json::Value, GraphError> {
        let messages = state
            .get("messages")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| GraphError::InvalidUpdate {
                message: "state must contain a 'messages' array".into(),
            })?;

        let last_msg = messages.last().ok_or_else(|| GraphError::InvalidUpdate {
            message: "messages array is empty".into(),
        })?;

        let tool_calls = last_msg
            .get("tool_calls")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| GraphError::InvalidUpdate {
                message: "last message has no 'tool_calls' array".into(),
            })?;

        if tool_calls.is_empty() {
            return Ok(state);
        }

        let mut tool_messages: Vec<serde_json::Value> = Vec::with_capacity(tool_calls.len());

        for tc in tool_calls {
            let call_id = tc
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let name = tc
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let arguments = tc
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

            let tool = self
                .tools
                .get(name)
                .ok_or_else(|| GraphError::ToolNotFound {
                    name: name.to_owned(),
                })?;

            // Validate arguments against the tool's JSON Schema (T302).
            if !tool.schema().parameters.is_null()
                && let Ok(validator) = jsonschema::validator_for(&tool.schema().parameters)
                && !validator.is_valid(&arguments)
            {
                return Err(GraphError::ToolInvocation {
                    tool: name.to_owned(),
                    message: "arguments failed JSON Schema validation".into(),
                });
            }

            // Resolve timeout: per-tool config takes precedence (T300).
            let timeout_duration = self
                .tool_configs
                .get(name)
                .and_then(|c| c.timeout)
                .or(self.default_timeout);
            let timeout_behavior = self
                .tool_configs
                .get(name)
                .map(|c| c.timeout_behavior)
                .unwrap_or_default();

            let output = if let Some(duration) = timeout_duration {
                match tokio::time::timeout(duration, tool.invoke(arguments)).await {
                    Ok(result) => result.map_err(|e| GraphError::ToolInvocation {
                        tool: name.to_owned(),
                        message: e.to_string(),
                    })?,
                    Err(_elapsed) => match timeout_behavior {
                        TimeoutBehavior::RaiseException => {
                            return Err(GraphError::ToolInvocation {
                                tool: name.to_owned(),
                                message: format!(
                                    "tool raised timeout exception after {duration:?}"
                                ),
                            });
                        }
                        // ReturnError and any future non_exhaustive variants.
                        _ => {
                            return Err(GraphError::ToolInvocation {
                                tool: name.to_owned(),
                                message: format!("tool timed out after {duration:?}"),
                            });
                        }
                    },
                }
            } else {
                tool.invoke(arguments)
                    .await
                    .map_err(|e| GraphError::ToolInvocation {
                        tool: name.to_owned(),
                        message: e.to_string(),
                    })?
            };

            let mut content = output.content;
            let truncated = if let Some(max_size) = self.max_result_size
                && content.len() > max_size
            {
                content.truncate(max_size);
                true
            } else {
                false
            };

            let entry = ToolResultEntry {
                tool_call_id: call_id.to_owned(),
                tool_name: name.to_owned(),
                content: content.clone(),
                truncated,
            };

            let tool_msg = serde_json::json!({
                "type": "tool",
                "content": entry.content,
                "tool_call_id": entry.tool_call_id,
                "name": entry.tool_name,
                "status": "success",
                "truncated": entry.truncated,
            });

            tool_messages.push(tool_msg);
        }

        // Append tool messages to state
        if let Some(arr) = state
            .get_mut("messages")
            .and_then(serde_json::Value::as_array_mut)
        {
            arr.extend(tool_messages);
        }

        Ok(state)
    }

    /// Converts this `ToolNode` into a [`NodeFn<MessagesState>`](crate::graph::state::NodeFn)
    /// suitable for use in a [`StateGraph<MessagesState>`](crate::graph::StateGraph).
    ///
    /// Internally converts `MessagesState` to `serde_json::Value`, runs the
    /// existing [`invoke`](Self::invoke), and converts back.
    pub fn into_messages_node_fn(
        self,
    ) -> crate::graph::state::NodeFn<crate::messages::MessagesState>
    where
        Self: 'static,
    {
        use crate::graph::state::State;
        let node = std::sync::Arc::new(self);
        Box::new(move |state: crate::messages::MessagesState| {
            let node = std::sync::Arc::clone(&node);
            Box::pin(async move {
                let value = state.to_value()?;
                let result = node.invoke(value).await?;
                crate::messages::MessagesState::from_value(result)
            })
        })
    }

    /// Converts this `ToolNode` into a [`NodeFn<ValueState>`](crate::graph::state::NodeFn)
    /// suitable for use in a [`StateGraph<ValueState>`](crate::graph::StateGraph).
    pub fn into_node_fn(self) -> crate::graph::state::NodeFn<crate::graph::ValueState>
    where
        Self: 'static,
    {
        let node = std::sync::Arc::new(self);
        Box::new(move |state: crate::graph::ValueState| {
            let node = std::sync::Arc::clone(&node);
            Box::pin(async move {
                let result = node.invoke(state.0).await?;
                Ok(crate::graph::ValueState(result))
            })
        })
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for ToolNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolNode")
            .field("tool_count", &self.tools.len())
            .field("max_result_size", &self.max_result_size)
            .field("default_timeout", &self.default_timeout)
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::unnecessary_literal_bound)]
mod tests {
    use super::*;
    use synwire_core::BoxFuture;
    use synwire_core::error::SynwireError;
    use synwire_core::tools::{ToolOutput, ToolSchema};

    struct MockTool {
        tool_name: String,
        schema: ToolSchema,
    }

    impl MockTool {
        fn new(name: &str) -> Self {
            Self {
                tool_name: name.to_owned(),
                schema: ToolSchema {
                    name: name.to_owned(),
                    description: format!("Mock tool: {name}"),
                    parameters: serde_json::json!({"type": "object"}),
                },
            }
        }
    }

    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.tool_name
        }
        fn description(&self) -> &str {
            &self.schema.description
        }
        fn schema(&self) -> &ToolSchema {
            &self.schema
        }
        fn invoke(
            &self,
            input: serde_json::Value,
        ) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
            Box::pin(async move {
                Ok(ToolOutput {
                    content: format!("result for {}: {input}", self.tool_name),
                    artifact: None,
                    ..ToolOutput::default()
                })
            })
        }
    }

    #[tokio::test]
    async fn test_tool_node_executes_tool_call() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockTool::new("search"))];
        let node = ToolNode::new(tools);

        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "Let me search",
                    "tool_calls": [
                        {
                            "id": "tc_1",
                            "name": "search",
                            "arguments": {"query": "rust"}
                        }
                    ]
                }
            ]
        });

        let result = node.invoke(state).await.unwrap();
        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1]["type"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "tc_1");
        assert!(messages[1]["content"].as_str().unwrap().contains("search"));
    }

    #[tokio::test]
    async fn test_tool_node_unknown_tool() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockTool::new("search"))];
        let node = ToolNode::new(tools);

        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "",
                    "tool_calls": [
                        {"id": "tc_1", "name": "unknown", "arguments": {}}
                    ]
                }
            ]
        });

        let err = node.invoke(state).await.unwrap_err();
        assert!(err.to_string().contains("tool not found: unknown"));
    }

    #[tokio::test]
    async fn test_tool_node_truncates_large_results() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockTool::new("search"))];
        let node = ToolNode::new(tools).with_max_result_size(10);

        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "",
                    "tool_calls": [
                        {"id": "tc_1", "name": "search", "arguments": {"query": "rust"}}
                    ]
                }
            ]
        });

        let result = node.invoke(state).await.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let tool_msg = &messages[1];
        assert!(tool_msg["content"].as_str().unwrap().len() <= 10);
        assert_eq!(tool_msg["truncated"], true);
    }

    #[tokio::test]
    async fn test_tool_node_no_messages_errors() {
        let node = ToolNode::new(vec![]);
        let state = serde_json::json!({});
        let err = node.invoke(state).await.unwrap_err();
        assert!(err.to_string().contains("messages"));
    }

    #[tokio::test]
    async fn test_tool_node_empty_tool_calls() {
        let node = ToolNode::new(vec![]);
        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "done",
                    "tool_calls": []
                }
            ]
        });
        let result = node.invoke(state.clone()).await.unwrap();
        assert_eq!(result, state);
    }

    #[tokio::test]
    async fn test_tool_node_multiple_tools() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(MockTool::new("search")),
            Box::new(MockTool::new("calc")),
        ];
        let node = ToolNode::new(tools);

        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "",
                    "tool_calls": [
                        {"id": "tc_1", "name": "search", "arguments": {"q": "a"}},
                        {"id": "tc_2", "name": "calc", "arguments": {"expr": "1+1"}}
                    ]
                }
            ]
        });

        let result = node.invoke(state).await.unwrap();
        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1]["tool_call_id"], "tc_1");
        assert_eq!(messages[2]["tool_call_id"], "tc_2");
    }

    #[tokio::test]
    async fn test_tool_node_timeout_returns_error() {
        use std::time::Duration;
        use synwire_core::tools::ToolConfig;

        struct SlowTool {
            schema: ToolSchema,
        }
        impl Tool for SlowTool {
            fn name(&self) -> &str {
                "slow"
            }
            fn description(&self) -> &str {
                "slow"
            }
            fn schema(&self) -> &ToolSchema {
                &self.schema
            }
            fn invoke(
                &self,
                _input: serde_json::Value,
            ) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
                Box::pin(async {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    Ok(ToolOutput::default())
                })
            }
        }

        let tools: Vec<Box<dyn Tool>> = vec![Box::new(SlowTool {
            schema: ToolSchema {
                name: "slow".into(),
                description: "slow".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        })];
        let config = ToolConfig {
            timeout: Some(Duration::from_millis(50)),
            ..ToolConfig::default()
        };
        let node = ToolNode::new(tools).with_tool_config("slow", config);

        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "",
                    "tool_calls": [
                        {"id": "tc_1", "name": "slow", "arguments": {}}
                    ]
                }
            ]
        });

        let err = node.invoke(state).await.unwrap_err();
        assert!(
            err.to_string().contains("timed out"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn test_tool_node_schema_validation_rejects_bad_args() {
        struct StrictTool {
            schema: ToolSchema,
        }
        impl Tool for StrictTool {
            fn name(&self) -> &str {
                "strict"
            }
            fn description(&self) -> &str {
                "strict"
            }
            fn schema(&self) -> &ToolSchema {
                &self.schema
            }
            fn invoke(
                &self,
                _input: serde_json::Value,
            ) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
                Box::pin(async { Ok(ToolOutput::default()) })
            }
        }

        let tools: Vec<Box<dyn Tool>> = vec![Box::new(StrictTool {
            schema: ToolSchema {
                name: "strict".into(),
                description: "strict".into(),
                // Requires a "query" field of type string.
                parameters: serde_json::json!({
                    "type": "object",
                    "required": ["query"],
                    "properties": {"query": {"type": "string"}},
                    "additionalProperties": false
                }),
            },
        })];
        let node = ToolNode::new(tools);

        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "",
                    "tool_calls": [
                        // Missing required "query" field.
                        {"id": "tc_1", "name": "strict", "arguments": {"bad": 42}}
                    ]
                }
            ]
        });

        let err = node.invoke(state).await.unwrap_err();
        assert!(
            err.to_string().contains("validation"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn test_tool_node_into_node_fn() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockTool::new("echo"))];
        let node_fn = ToolNode::new(tools).into_node_fn();

        let state = serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "",
                    "tool_calls": [
                        {"id": "tc_1", "name": "echo", "arguments": {"text": "hi"}}
                    ]
                }
            ]
        });

        let result = node_fn(crate::graph::ValueState(state)).await.unwrap();
        let messages = result.0["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
    }
}

//! `ReAct` agent pattern.
//!
//! Implements the Reason + Act loop as a [`StateGraph`]:
//!
//! ```text
//! START -> agent (LLM) -> [tools_condition] -> tools (ToolNode) -> agent ...
//!                        \-> END (no tool calls)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;
use synwire_core::tools::{Tool, ToolSchema};

use crate::constants::END;
use crate::error::GraphError;
use crate::graph::compiled::CompiledGraph;
use crate::graph::state::StateGraph;
use crate::graph::value_state::ValueState;
use crate::messages::MessagesState;
use crate::prebuilt::tool_node::ToolNode;

/// Key returned by [`tools_condition`] when tool calls are present.
const TOOLS_KEY: &str = "tools";

/// Condition function: routes to `"tools"` if the last AI message contains
/// tool calls, or to [`END`] otherwise.
///
/// Inspects `state["messages"]` for the last message. If it has a non-empty
/// `"tool_calls"` array, returns `"tools"`. Otherwise returns [`END`].
pub fn tools_condition(state: &ValueState) -> String {
    let has_tool_calls = state
        .0
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .and_then(|msgs| msgs.last())
        .and_then(|msg| msg.get("tool_calls"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|tc| !tc.is_empty());

    if has_tool_calls {
        TOOLS_KEY.to_owned()
    } else {
        END.to_owned()
    }
}

/// Creates a `ReAct` agent as a compiled [`StateGraph<ValueState>`].
///
/// The resulting graph has three nodes:
/// - `"agent"`: invokes the chat model with the current messages
/// - `"tools"`: executes any tool calls from the agent's response
/// - A conditional edge from `"agent"` that routes to `"tools"` or `END`
///
/// # State contract
///
/// The input state must contain a `"messages"` key with an array of message
/// objects. The agent appends AI and tool messages as execution proceeds.
///
/// # Errors
///
/// - [`GraphError::Core`] if `bind_tools` is not supported by the model.
/// - [`GraphError::CompileError`] if graph construction fails.
#[allow(clippy::needless_pass_by_value)] // Box<dyn Trait> is the idiomatic owned trait object
pub fn create_react_agent(
    model: Box<dyn BaseChatModel>,
    tools: Vec<Box<dyn Tool>>,
) -> Result<CompiledGraph<ValueState>, GraphError> {
    // Collect tool schemas for model binding.
    let schemas: Vec<ToolSchema> = tools.iter().map(|t| t.schema().clone()).collect();

    // Bind tools to model so it knows which tools are available.
    let bound_model: Arc<Box<dyn BaseChatModel>> =
        Arc::new(model.bind_tools(&schemas).map_err(GraphError::Core)?);

    // Build the agent node: invokes the model and appends its response.
    let model_ref = Arc::clone(&bound_model);
    let agent_fn: crate::graph::state::NodeFn<ValueState> = Box::new(move |state: ValueState| {
        let model = Arc::clone(&model_ref);
        Box::pin(async move {
            let messages: Vec<Message> = state
                .0
                .get("messages")
                .and_then(serde_json::Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| serde_json::from_value(v.clone()).ok())
                        .collect()
                })
                .unwrap_or_default();

            let result = model
                .invoke(&messages, None)
                .await
                .map_err(GraphError::Core)?;

            let response_json =
                serde_json::to_value(&result.message).map_err(|e| GraphError::InvalidUpdate {
                    message: format!("failed to serialise AI message: {e}"),
                })?;

            let mut new_state = state;
            if let Some(arr) = new_state
                .0
                .get_mut("messages")
                .and_then(serde_json::Value::as_array_mut)
            {
                arr.push(response_json);
            }

            Ok(new_state)
        })
    });

    // Build the tools node.
    let tool_node = ToolNode::new(tools);
    let tools_fn = tool_node.into_node_fn();

    // Assemble the graph.
    let mut graph = StateGraph::<ValueState>::new();
    let _ = graph.add_node("agent", agent_fn)?;
    let _ = graph.add_node("tools", tools_fn)?;

    let _g = graph.set_entry_point("agent");

    let mut mapping = HashMap::new();
    let _ins1 = mapping.insert(TOOLS_KEY.to_owned(), "tools".to_owned());
    let _ins2 = mapping.insert(END.to_owned(), END.to_owned());
    let _g = graph.add_conditional_edges("agent", Box::new(tools_condition), mapping);
    let _g = graph.add_edge("tools", "agent");

    graph.compile()
}

/// Condition function for [`MessagesState`]: routes to `"tools"` if the last
/// AI message contains tool calls, or to [`END`] otherwise.
///
/// Inspects `state.messages` directly (no JSON parsing).
pub fn messages_tools_condition(state: &MessagesState) -> String {
    let has_tool_calls = state
        .messages
        .last()
        .is_some_and(|msg| matches!(msg, Message::AI { tool_calls, .. } if !tool_calls.is_empty()));

    if has_tool_calls {
        TOOLS_KEY.to_owned()
    } else {
        END.to_owned()
    }
}

/// Creates a `ReAct` agent as a compiled [`StateGraph<MessagesState>`].
///
/// Identical to [`create_react_agent`] but uses the typed [`MessagesState`]
/// instead of [`ValueState`]. The agent node works directly with
/// `state.messages` and the tool node bridges through JSON serialisation.
///
/// # Errors
///
/// - [`GraphError::Core`] if `bind_tools` is not supported by the model.
/// - [`GraphError::CompileError`] if graph construction fails.
#[allow(clippy::needless_pass_by_value)]
pub fn create_react_agent_messages(
    model: Box<dyn BaseChatModel>,
    tools: Vec<Box<dyn Tool>>,
) -> Result<CompiledGraph<MessagesState>, GraphError> {
    let schemas: Vec<ToolSchema> = tools.iter().map(|t| t.schema().clone()).collect();
    let bound_model: Arc<Box<dyn BaseChatModel>> =
        Arc::new(model.bind_tools(&schemas).map_err(GraphError::Core)?);

    // Agent node: invokes the model with state.messages, appends AI response.
    let model_ref = Arc::clone(&bound_model);
    let agent_fn: crate::graph::state::NodeFn<MessagesState> =
        Box::new(move |state: MessagesState| {
            let model = Arc::clone(&model_ref);
            Box::pin(async move {
                let result = model
                    .invoke(&state.messages, None)
                    .await
                    .map_err(GraphError::Core)?;

                let mut new_state = state;
                new_state.messages.push(result.message);
                Ok(new_state)
            })
        });

    // Tool node: bridge MessagesState through JSON for tool execution.
    let tool_node = ToolNode::new(tools);
    let tools_fn = tool_node.into_messages_node_fn();

    let mut graph = StateGraph::<MessagesState>::new();
    let _ = graph.add_node("agent", agent_fn)?;
    let _ = graph.add_node("tools", tools_fn)?;

    let _g = graph.set_entry_point("agent");

    let mut mapping = HashMap::new();
    let _ins1 = mapping.insert(TOOLS_KEY.to_owned(), "tools".to_owned());
    let _ins2 = mapping.insert(END.to_owned(), END.to_owned());
    let _g = graph.add_conditional_edges("agent", Box::new(messages_tools_condition), mapping);
    let _g = graph.add_edge("tools", "agent");

    graph.compile()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_condition_with_tool_calls() {
        let state = ValueState(serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "Let me search",
                    "tool_calls": [
                        {"id": "tc_1", "name": "search", "arguments": {}}
                    ]
                }
            ]
        }));
        assert_eq!(tools_condition(&state), "tools");
    }

    #[test]
    fn test_tools_condition_without_tool_calls() {
        let state = ValueState(serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "Here is your answer"
                }
            ]
        }));
        assert_eq!(tools_condition(&state), END);
    }

    #[test]
    fn test_tools_condition_empty_tool_calls() {
        let state = ValueState(serde_json::json!({
            "messages": [
                {
                    "type": "ai",
                    "content": "done",
                    "tool_calls": []
                }
            ]
        }));
        assert_eq!(tools_condition(&state), END);
    }

    #[test]
    fn test_tools_condition_no_messages() {
        let state = ValueState(serde_json::json!({}));
        assert_eq!(tools_condition(&state), END);
    }

    #[test]
    fn test_tools_condition_empty_messages() {
        let state = ValueState(serde_json::json!({"messages": []}));
        assert_eq!(tools_condition(&state), END);
    }

    #[tokio::test]
    async fn test_create_react_agent_compiles() {
        use synwire_core::language_models::FakeChatModel;

        // FakeChatModel supports bind_tools, so this should compile.
        let model: Box<dyn BaseChatModel> = Box::new(FakeChatModel::new(vec!["Hello".into()]));
        let tools: Vec<Box<dyn Tool>> = vec![];

        let graph = create_react_agent(model, tools).unwrap();
        assert!(graph.node_names().contains(&"agent"));
        assert!(graph.node_names().contains(&"tools"));
    }

    #[tokio::test]
    async fn test_react_agent_no_tool_calls_terminates() {
        use synwire_core::language_models::FakeChatModel;

        // The fake model returns plain text (no tool_calls), so the agent
        // should go agent -> END in one step.
        let model: Box<dyn BaseChatModel> =
            Box::new(FakeChatModel::new(vec!["The answer is 42".into()]));
        let tools: Vec<Box<dyn Tool>> = vec![];

        let graph = create_react_agent(model, tools).unwrap();
        let state = ValueState(serde_json::json!({
            "messages": [
                {"type": "human", "content": "What is the answer?"}
            ]
        }));

        let result = graph.invoke(state).await.unwrap();
        let messages = result.0["messages"].as_array().unwrap();
        // Should have: human + ai response
        assert_eq!(messages.len(), 2);
    }

    // ---- MessagesState-based tests ----

    #[test]
    fn test_messages_tools_condition_with_tool_calls() {
        use synwire_core::messages::{MessageContent, ToolCall};

        let state = MessagesState {
            messages: vec![Message::AI {
                id: None,
                name: None,
                content: MessageContent::Text("Let me search".into()),
                tool_calls: vec![ToolCall {
                    id: "tc_1".into(),
                    name: "search".into(),
                    arguments: HashMap::default(),
                }],
                invalid_tool_calls: vec![],
                usage: None,
                response_metadata: None,
                additional_kwargs: HashMap::default(),
            }],
        };
        assert_eq!(messages_tools_condition(&state), "tools");
    }

    #[test]
    fn test_messages_tools_condition_without_tool_calls() {
        use synwire_core::messages::MessageContent;

        let state = MessagesState {
            messages: vec![Message::AI {
                id: None,
                name: None,
                content: MessageContent::Text("Here is your answer".into()),
                tool_calls: vec![],
                invalid_tool_calls: vec![],
                usage: None,
                response_metadata: None,
                additional_kwargs: HashMap::default(),
            }],
        };
        assert_eq!(messages_tools_condition(&state), END);
    }

    #[test]
    fn test_messages_tools_condition_empty() {
        let state = MessagesState { messages: vec![] };
        assert_eq!(messages_tools_condition(&state), END);
    }

    /// T025: `create_react_agent_messages` returns `CompiledGraph<MessagesState>`.
    #[tokio::test]
    async fn t025_create_react_agent_messages_compiles() {
        use synwire_core::language_models::FakeChatModel;

        let model: Box<dyn BaseChatModel> = Box::new(FakeChatModel::new(vec!["Hello".into()]));
        let tools: Vec<Box<dyn Tool>> = vec![];

        let graph = create_react_agent_messages(model, tools).unwrap();
        assert!(graph.node_names().contains(&"agent"));
        assert!(graph.node_names().contains(&"tools"));
    }

    /// T027: `ReAct` agent with no tool calls terminates and returns `MessagesState`.
    #[tokio::test]
    async fn t027_react_agent_messages_no_tool_calls_terminates() {
        use synwire_core::language_models::FakeChatModel;
        use synwire_core::messages::MessageContent;

        let model: Box<dyn BaseChatModel> =
            Box::new(FakeChatModel::new(vec!["The answer is 42".into()]));
        let tools: Vec<Box<dyn Tool>> = vec![];

        let graph = create_react_agent_messages(model, tools).unwrap();
        let state = MessagesState {
            messages: vec![Message::Human {
                id: None,
                name: None,
                content: MessageContent::Text("What is the answer?".into()),
                additional_kwargs: HashMap::default(),
            }],
        };

        let result = graph.invoke(state).await.unwrap();
        // Should have: human + ai response
        assert_eq!(result.messages.len(), 2);

        // First message is the human input.
        assert!(matches!(&result.messages[0], Message::Human { .. }));
        // Second message is the AI response.
        assert!(matches!(&result.messages[1], Message::AI { .. }));
    }
}

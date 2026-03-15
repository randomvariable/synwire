//! Prebuilt agents and nodes for common orchestration patterns.
//!
//! This module provides ready-to-use graph components:
//!
//! - [`ToolNode`] -- executes tool calls extracted from state
//! - [`create_react_agent`] -- builds a ReAct-style agent graph
//! - [`IfElseNode`], [`LoopNode`], [`HttpRequestNode`] -- control flow primitives
//! - [`ValidationNode`], [`TemplateTransformNode`], [`ListOperatorNode`] -- data transforms
//! - [`VariableAggregatorNode`], [`QuestionClassifierNode`], [`IterationNode`] -- advanced patterns

mod nodes;
mod react_agent;
mod tool_node;

pub use nodes::{
    HttpRequestNode, IfElseNode, IterationNode, ListOperatorNode, LoopNode, QuestionClassifierNode,
    TemplateTransformNode, ValidationNode, VariableAggregatorNode,
};
pub use react_agent::{
    create_react_agent, create_react_agent_messages, messages_tools_condition, tools_condition,
};
pub use tool_node::{ToolNode, ToolResultEntry};

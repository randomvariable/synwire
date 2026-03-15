//! Agent types for execution steps and decisions.

/// An action to be taken by an agent.
#[derive(Debug, Clone)]
pub struct AgentAction {
    /// Tool name to invoke.
    pub tool: String,
    /// Tool input arguments.
    pub tool_input: serde_json::Value,
    /// Log of reasoning leading to this action.
    pub log: String,
}

/// The final result from an agent.
#[derive(Debug, Clone)]
pub struct AgentFinish {
    /// The final output.
    pub return_values: serde_json::Map<String, serde_json::Value>,
    /// Log of reasoning.
    pub log: String,
}

/// A single step in agent execution.
#[derive(Debug, Clone)]
pub struct AgentStep {
    /// The action that was taken.
    pub action: AgentAction,
    /// The observation from executing the action.
    pub observation: String,
}

/// Decision made by an agent at each step.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AgentDecision {
    /// Agent decided to take one or more actions.
    Action(Vec<AgentAction>),
    /// Agent decided to finish.
    Finish(AgentFinish),
}

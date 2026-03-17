//! Agent node trait, builder, and execution context.
//!
//! `AgentNode` is the primary abstraction for a runnable agent.  The `Agent<D>`
//! builder configures all agent parameters and implements `AgentNode`.

use std::collections::HashMap;
use std::pin::Pin;

use futures_core::Stream;
use serde::Serialize;
use serde_json::Value;

use crate::BoxFuture;
use crate::agents::error::AgentError;
use crate::agents::hooks::HookRegistry;
use crate::agents::middleware::MiddlewareStack;
use crate::agents::model_info::{EffortLevel, ThinkingConfig};
use crate::agents::output_mode::SystemPromptConfig;
use crate::agents::permission::{PermissionMode, PermissionRule};
use crate::agents::plugin::Plugin;
use crate::agents::sandbox::SandboxConfig;
use crate::agents::streaming::AgentEvent;
use crate::tools::Tool;
use crate::vfs::OutputFormat;

/// Stream of agent events produced during a run.
pub type AgentEventStream = Pin<Box<dyn Stream<Item = Result<AgentEvent, AgentError>> + Send>>;

// ---------------------------------------------------------------------------
// AgentNode trait
// ---------------------------------------------------------------------------

/// A runnable agent that produces a stream of events.
pub trait AgentNode: Send + Sync {
    /// Unique agent name (stable identifier for routing and logging).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Run the agent, returning a stream of events.
    ///
    /// `input` is the initial user message or continuation prompt.
    /// The stream ends with a `TurnComplete` or `Error` event.
    fn run(&self, input: Value) -> BoxFuture<'_, Result<AgentEventStream, AgentError>>;

    /// Returns the names of agents this node may spawn as sub-agents.
    fn sub_agents(&self) -> Vec<String> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// OutputMode
// ---------------------------------------------------------------------------

/// Configures how the agent extracts structured output from the model response.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum OutputMode {
    /// Structured output via tool call (most reliable).
    #[default]
    Tool,
    /// Native JSON mode (provider must support it).
    Native,
    /// Post-process raw text response via prompt.
    Prompt,
    /// Custom extraction via user-supplied function.
    Custom,
}

// ---------------------------------------------------------------------------
// RunContext
// ---------------------------------------------------------------------------

/// Runtime context made available to agent execution.
#[derive(Debug)]
pub struct RunContext {
    /// Session identifier.
    pub session_id: Option<String>,
    /// Model name resolved for this run.
    pub model: String,
    /// Retry count for the current turn (0 = first attempt).
    pub retry_count: u32,
    /// Cumulative cost so far in this session (USD).
    pub cumulative_cost_usd: f64,
    /// Arbitrary metadata attached at call site.
    pub metadata: HashMap<String, Value>,
}

impl RunContext {
    /// Create a new context with default values.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            session_id: None,
            model: model.into(),
            retry_count: 0,
            cumulative_cost_usd: 0.0,
            metadata: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Callbacks
// ---------------------------------------------------------------------------

/// Called immediately before the agent starts processing a turn.
pub type BeforeAgentCallback =
    Box<dyn Fn(&RunContext) -> BoxFuture<'static, Result<(), AgentError>> + Send + Sync>;

/// Called after the agent completes a turn (success or failure).
pub type AfterAgentCallback =
    Box<dyn Fn(&RunContext, &Result<(), AgentError>) -> BoxFuture<'static, ()> + Send + Sync>;

/// Called when a model error occurs, allowing custom recovery logic.
pub type OnModelErrorCallback =
    Box<dyn Fn(&RunContext, &AgentError) -> BoxFuture<'static, ModelErrorAction> + Send + Sync>;

/// Recovery action returned by `OnModelErrorCallback`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ModelErrorAction {
    /// Retry the current request unchanged.
    Retry,
    /// Abort the run with the given error.
    Abort(String),
    /// Switch to a different model and retry.
    SwitchModel(String),
}

// ---------------------------------------------------------------------------
// Agent builder
// ---------------------------------------------------------------------------

/// Builder for configuring and constructing a runnable agent.
///
/// Type parameter `O` is the optional structured output type (use `()` for
/// unstructured text).
#[derive(Default)]
#[allow(clippy::struct_field_names)]
pub struct Agent<O: Serialize + Send + Sync + 'static = ()> {
    // Identity
    name: String,
    description: String,

    // Model
    model: String,
    fallback_model: Option<String>,
    effort: Option<EffortLevel>,
    thinking: Option<ThinkingConfig>,

    // Tools
    tools: Vec<Box<dyn Tool>>,
    allowed_tools: Option<Vec<String>>,
    excluded_tools: Vec<String>,

    // Plugins
    plugins: Vec<Box<dyn Plugin>>,

    // Middleware
    middleware: MiddlewareStack,

    // Hooks
    hooks: HookRegistry,

    // Output
    output_mode: OutputMode,
    output_schema: Option<Value>,

    // Tool output serialization — controls how VFS / tool results are
    // formatted before being passed back to the LLM.
    tool_output_format: OutputFormat,

    // Limits
    max_turns: Option<u32>,
    max_budget: Option<f64>,

    // System prompt
    system_prompt: Option<SystemPromptConfig>,

    // Permissions
    permission_mode: PermissionMode,
    permission_rules: Vec<PermissionRule>,

    // Sandbox
    sandbox: Option<SandboxConfig>,

    // Environment
    env: HashMap<String, String>,
    cwd: Option<String>,

    // Debug
    debug: bool,
    debug_file: Option<String>,

    // MCP
    mcp_servers: Vec<String>,

    // Callbacks
    before_agent: Option<BeforeAgentCallback>,
    after_agent: Option<AfterAgentCallback>,
    on_model_error: Option<OnModelErrorCallback>,

    _output: std::marker::PhantomData<O>,
}

impl<O: Serialize + Send + Sync + 'static> std::fmt::Debug for Agent<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("name", &self.name)
            .field("model", &self.model)
            .field("max_turns", &self.max_turns)
            .field("max_budget", &self.max_budget)
            .finish_non_exhaustive()
    }
}

impl<O: Serialize + Send + Sync + 'static> Agent<O> {
    /// Create a new agent builder.
    #[must_use]
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            ..Self::default()
        }
    }

    /// Set a human-readable description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the primary model name.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set a fallback model used when the primary is rate-limited or unavailable.
    #[must_use]
    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.fallback_model = Some(model.into());
        self
    }

    /// Set the reasoning effort level.
    #[must_use]
    pub const fn effort(mut self, effort: EffortLevel) -> Self {
        self.effort = Some(effort);
        self
    }

    /// Configure extended thinking / chain-of-thought.
    #[must_use]
    pub const fn thinking(mut self, thinking: ThinkingConfig) -> Self {
        self.thinking = Some(thinking);
        self
    }

    /// Add a tool available to the agent.
    #[must_use]
    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    /// Restrict the agent to only these tool names (allowlist).
    #[must_use]
    pub fn allowed_tools(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allowed_tools = Some(tools.into_iter().map(Into::into).collect());
        self
    }

    /// Exclude specific tools by name (denylist within allowlist).
    #[must_use]
    pub fn exclude_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.excluded_tools.push(tool_name.into());
        self
    }

    /// Add a plugin.
    #[must_use]
    pub fn plugin(mut self, plugin: impl Plugin + 'static) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Add a middleware component.
    #[must_use]
    pub fn middleware(mut self, mw: impl crate::agents::middleware::Middleware + 'static) -> Self {
        self.middleware.push(mw);
        self
    }

    /// Configure hooks.
    #[must_use]
    pub fn hooks(mut self, hooks: HookRegistry) -> Self {
        self.hooks = hooks;
        self
    }

    /// Set the structured output mode.
    #[must_use]
    pub const fn output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }

    /// Provide a JSON Schema for structured output validation.
    #[must_use]
    pub fn output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Set the default serialization format for tool and VFS output.
    ///
    /// Controls how structured data is rendered before being passed back
    /// to the LLM as tool results.  Defaults to [`OutputFormat::Json`].
    /// Individual tools can override this per-call.
    #[must_use]
    pub const fn tool_output_format(mut self, format: OutputFormat) -> Self {
        self.tool_output_format = format;
        self
    }

    /// Returns the configured tool output format.
    #[must_use]
    pub const fn get_tool_output_format(&self) -> OutputFormat {
        self.tool_output_format
    }

    /// Set the maximum number of agent turns per run.
    #[must_use]
    pub const fn max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Set the maximum cumulative cost budget (USD).
    #[must_use]
    pub const fn max_budget(mut self, budget_usd: f64) -> Self {
        self.max_budget = Some(budget_usd);
        self
    }

    /// Configure the system prompt.
    #[must_use]
    pub fn system_prompt(mut self, config: SystemPromptConfig) -> Self {
        self.system_prompt = Some(config);
        self
    }

    /// Set the permission mode.
    #[must_use]
    pub const fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Add a permission rule.
    #[must_use]
    pub fn permission_rule(mut self, rule: PermissionRule) -> Self {
        self.permission_rules.push(rule);
        self
    }

    /// Configure the sandbox.
    #[must_use]
    pub fn sandbox(mut self, config: SandboxConfig) -> Self {
        self.sandbox = Some(config);
        self
    }

    /// Set an environment variable available to the agent.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let _ = self.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory for the agent.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Enable debug mode (verbose logging).
    #[must_use]
    pub const fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Write debug output to a file.
    #[must_use]
    pub fn debug_file(mut self, path: impl Into<String>) -> Self {
        self.debug_file = Some(path.into());
        self
    }

    /// Register an MCP server by name.
    #[must_use]
    pub fn mcp_server(mut self, server_name: impl Into<String>) -> Self {
        self.mcp_servers.push(server_name.into());
        self
    }

    /// Register a before-agent callback.
    #[must_use]
    pub fn before_agent<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunContext) -> BoxFuture<'static, Result<(), AgentError>> + Send + Sync + 'static,
    {
        self.before_agent = Some(Box::new(f));
        self
    }

    /// Register an after-agent callback.
    #[must_use]
    pub fn after_agent<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunContext, &Result<(), AgentError>) -> BoxFuture<'static, ()>
            + Send
            + Sync
            + 'static,
    {
        self.after_agent = Some(Box::new(f));
        self
    }

    /// Register a model error callback.
    #[must_use]
    pub fn on_model_error<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunContext, &AgentError) -> BoxFuture<'static, ModelErrorAction>
            + Send
            + Sync
            + 'static,
    {
        self.on_model_error = Some(Box::new(f));
        self
    }

    // --- Accessors (for the runner) ---

    /// Agent name.
    #[must_use]
    pub fn agent_name(&self) -> &str {
        &self.name
    }

    /// Agent description.
    #[must_use]
    pub fn agent_description(&self) -> &str {
        &self.description
    }

    /// Primary model identifier.
    #[must_use]
    pub fn model_name(&self) -> &str {
        &self.model
    }

    /// Fallback model identifier, if configured.
    #[must_use]
    pub fn fallback_model_name(&self) -> Option<&str> {
        self.fallback_model.as_deref()
    }

    /// Maximum turns, if set.
    #[must_use]
    pub const fn max_turn_count(&self) -> Option<u32> {
        self.max_turns
    }

    /// Maximum budget (USD), if set.
    #[must_use]
    pub const fn budget_limit(&self) -> Option<f64> {
        self.max_budget
    }

    /// Whether debug mode is enabled.
    #[must_use]
    pub const fn is_debug(&self) -> bool {
        self.debug
    }
}

impl<O: Serialize + Send + Sync + 'static> Agent<O> {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            model: String::new(),
            fallback_model: None,
            effort: None,
            thinking: None,
            tools: Vec::new(),
            allowed_tools: None,
            excluded_tools: Vec::new(),
            plugins: Vec::new(),
            middleware: MiddlewareStack::new(),
            hooks: HookRegistry::new(),
            output_mode: OutputMode::default(),
            output_schema: None,
            tool_output_format: OutputFormat::Json,
            max_turns: None,
            max_budget: None,
            system_prompt: None,
            permission_mode: PermissionMode::default(),
            permission_rules: Vec::new(),
            sandbox: None,
            env: HashMap::new(),
            cwd: None,
            debug: false,
            debug_file: None,
            mcp_servers: Vec::new(),
            before_agent: None,
            after_agent: None,
            on_model_error: None,
            _output: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_fields() {
        let agent: Agent = Agent::new("my-agent", "claude-3-5-sonnet")
            .description("Test agent")
            .max_turns(10)
            .max_budget(1.0)
            .fallback_model("claude-3-haiku");

        assert_eq!(agent.agent_name(), "my-agent");
        assert_eq!(agent.model_name(), "claude-3-5-sonnet");
        assert_eq!(agent.max_turn_count(), Some(10));
        assert_eq!(agent.budget_limit(), Some(1.0));
        assert_eq!(agent.fallback_model_name(), Some("claude-3-haiku"));
    }
}

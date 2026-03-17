//! Environment middleware — exposes environment variable operations as agent tools.

use synwire_core::agents::middleware::Middleware;
use synwire_core::tools::Tool;

/// Middleware that exposes environment variable operations to the agent.
///
/// Provides read access to environment variables and optionally write access
/// for setting per-agent environment state.
#[derive(Debug, Default)]
pub struct EnvironmentMiddleware;

impl Middleware for EnvironmentMiddleware {
    fn name(&self) -> &'static str {
        "environment"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        Vec::new()
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec![
            "You have access to environment tools: get_env, set_env, list_env. \
             Use get_env to read environment variable values, set_env to configure \
             per-session variables, and list_env to enumerate available variables."
                .to_string(),
        ]
    }
}

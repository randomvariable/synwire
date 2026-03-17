//! Pipeline middleware — exposes pipeline composition as agent tools.

use synwire_core::agents::middleware::Middleware;
use synwire_core::tools::Tool;

/// Middleware that exposes pipeline execution tools to the agent.
#[derive(Debug, Default)]
pub struct PipelineMiddleware;

impl Middleware for PipelineMiddleware {
    fn name(&self) -> &'static str {
        "pipeline"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        Vec::new()
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec![
            "You have access to pipeline tools: execute_pipeline. Pipe commands together with stdin/stdout redirection.".to_string(),
        ]
    }
}

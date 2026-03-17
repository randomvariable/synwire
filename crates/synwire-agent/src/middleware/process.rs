//! Process middleware — exposes process management as agent tools.

use synwire_core::agents::middleware::Middleware;
use synwire_core::tools::Tool;

/// Middleware that exposes process management tools with safety guards.
#[derive(Debug, Default)]
pub struct ProcessMiddleware;

impl Middleware for ProcessMiddleware {
    fn name(&self) -> &'static str {
        "process"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        Vec::new()
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec![
            "You have access to process tools: list_processes, kill_process, spawn_background, list_jobs, execute_command.".to_string(),
            "Warning: Use process operations carefully. Killing system processes may cause instability.".to_string(),
        ]
    }
}

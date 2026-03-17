//! Git middleware — exposes git operations as agent tools.

use synwire_core::agents::middleware::Middleware;
use synwire_core::tools::Tool;

/// Middleware that exposes git operations to the agent.
#[derive(Debug, Default)]
pub struct GitMiddleware;

impl Middleware for GitMiddleware {
    fn name(&self) -> &'static str {
        "git"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        Vec::new()
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec![
            "You have access to git tools: git_status, git_diff, git_log, git_commit, git_push, git_pull, git_branch.".to_string(),
        ]
    }
}

//! Archive middleware — exposes archive operations as agent tools.

use synwire_core::agents::middleware::Middleware;
use synwire_core::tools::Tool;

/// Middleware that exposes archive tools to the agent.
#[derive(Debug, Default)]
pub struct ArchiveMiddleware;

impl Middleware for ArchiveMiddleware {
    fn name(&self) -> &'static str {
        "archive"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        Vec::new()
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec![
            "You have access to archive tools: create_archive, extract_archive, list_archive. Supported formats: tar, tar.gz, zip.".to_string(),
        ]
    }
}

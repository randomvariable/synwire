//! Filesystem middleware — exposes backend file operations as agent tools.

use synwire_core::agents::middleware::Middleware;
use synwire_core::tools::Tool;

/// Middleware that registers filesystem tools into the agent context.
///
/// Tools exposed: `ls`, `read_file`, `write_file`, `edit_file`, `rm`, `pwd`, `cd`.
#[derive(Debug, Default)]
pub struct FilesystemMiddleware;

impl Middleware for FilesystemMiddleware {
    fn name(&self) -> &'static str {
        "filesystem"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        // Tool implementations are wired up at runtime by the runner,
        // which injects the configured backend.  Here we return an empty
        // list; the runner collects tools from middleware by name.
        Vec::new()
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec![
            "You have access to filesystem tools: ls, read_file, write_file, edit_file, rm, pwd, cd.".to_string(),
        ]
    }
}

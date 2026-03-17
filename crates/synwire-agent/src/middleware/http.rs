//! HTTP middleware — exposes web request operations as agent tools.

use synwire_core::agents::middleware::Middleware;
use synwire_core::tools::Tool;

/// Middleware that exposes HTTP request tools to the agent.
#[derive(Debug, Default)]
pub struct HttpMiddleware;

impl Middleware for HttpMiddleware {
    fn name(&self) -> &'static str {
        "http"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        Vec::new()
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec![
            "You have access to HTTP tools: http_get, http_post, http_put, http_delete."
                .to_string(),
        ]
    }
}

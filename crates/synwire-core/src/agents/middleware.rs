//! Middleware stack for cross-cutting concerns.

use serde_json::Value;

use crate::BoxFuture;
use crate::agents::error::AgentError;
use crate::tools::Tool;

/// Input passed through the middleware chain.
#[derive(Debug, Clone)]
pub struct MiddlewareInput {
    /// Conversation messages as JSON values.
    pub messages: Vec<Value>,
    /// Arbitrary context metadata.
    pub context: Value,
}

/// Outcome from a middleware component.
#[derive(Debug)]
#[non_exhaustive]
pub enum MiddlewareResult {
    /// Pass the (possibly modified) input to the next middleware.
    Continue(MiddlewareInput),
    /// Terminate the chain immediately with a message.
    Terminate(String),
}

/// Cross-cutting concern injected into the agent loop.
pub trait Middleware: Send + Sync {
    /// Middleware identifier (for logging and ordering).
    fn name(&self) -> &str;

    /// Process the input and optionally call through to the next layer.
    ///
    /// The default implementation calls `next` unchanged.
    fn process(
        &self,
        input: MiddlewareInput,
    ) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
        Box::pin(async move { Ok(MiddlewareResult::Continue(input)) })
    }

    /// Tools injected into the agent context by this middleware.
    fn tools(&self) -> Vec<Box<dyn Tool>> {
        Vec::new()
    }

    /// System prompt additions contributed by this middleware.
    ///
    /// Additions are concatenated in stack order.
    fn system_prompt_additions(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Executes a slice of middleware in order.
///
/// - If any middleware returns `Terminate`, the chain stops.
/// - System prompt additions are collected from all middleware in order.
/// - Tools are collected from all middleware in order.
pub struct MiddlewareStack {
    components: Vec<Box<dyn Middleware>>,
}

impl std::fmt::Debug for MiddlewareStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiddlewareStack")
            .field(
                "components",
                &self.components.iter().map(|m| m.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl MiddlewareStack {
    /// Create an empty middleware stack.
    #[must_use]
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Append a middleware component to the stack.
    pub fn push(&mut self, middleware: impl Middleware + 'static) {
        self.components.push(Box::new(middleware));
    }

    /// Run the input through all middleware in order.
    pub async fn run(&self, mut input: MiddlewareInput) -> Result<MiddlewareResult, AgentError> {
        for mw in &self.components {
            match mw.process(input).await? {
                MiddlewareResult::Continue(next_input) => input = next_input,
                term @ MiddlewareResult::Terminate(_) => return Ok(term),
            }
        }
        Ok(MiddlewareResult::Continue(input))
    }

    /// Collect all system prompt additions from all middleware in order.
    #[must_use]
    pub fn system_prompt_additions(&self) -> Vec<String> {
        self.components
            .iter()
            .flat_map(|m| m.system_prompt_additions())
            .collect()
    }

    /// Collect all tools from all middleware in order.
    pub fn tools(&self) -> Vec<Box<dyn Tool>> {
        self.components.iter().flat_map(|m| m.tools()).collect()
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unnecessary_literal_bound
)]
mod tests {
    use super::*;

    struct OrderRecorder {
        name: &'static str,
        order: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
    }

    impl Middleware for OrderRecorder {
        fn name(&self) -> &str {
            self.name
        }

        fn process(
            &self,
            input: MiddlewareInput,
        ) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
            let order = self.order.clone();
            Box::pin(async move {
                if let Ok(mut g) = order.lock() {
                    g.push(self.name);
                }
                Ok(MiddlewareResult::Continue(input))
            })
        }

        fn system_prompt_additions(&self) -> Vec<String> {
            vec![format!("[{}]", self.name)]
        }
    }

    struct EarlyTerminator;
    impl Middleware for EarlyTerminator {
        fn name(&self) -> &str {
            "terminator"
        }
        fn process(
            &self,
            _input: MiddlewareInput,
        ) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
            Box::pin(async { Ok(MiddlewareResult::Terminate("stop".to_string())) })
        }
    }

    fn base_input() -> MiddlewareInput {
        MiddlewareInput {
            messages: Vec::new(),
            context: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_stack_order() {
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut stack = MiddlewareStack::new();
        stack.push(OrderRecorder {
            name: "a",
            order: order.clone(),
        });
        stack.push(OrderRecorder {
            name: "b",
            order: order.clone(),
        });
        let _ = stack.run(base_input()).await.expect("run");
        let seen = order.lock().expect("lock").clone();
        assert_eq!(seen, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn test_early_termination() {
        let mut stack = MiddlewareStack::new();
        stack.push(EarlyTerminator);
        // Second middleware should NOT run.
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        stack.push(OrderRecorder {
            name: "after",
            order: order.clone(),
        });
        let result = stack.run(base_input()).await.expect("run");
        assert!(matches!(result, MiddlewareResult::Terminate(_)));
        assert!(order.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn test_system_prompt_composition_order() {
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut stack = MiddlewareStack::new();
        stack.push(OrderRecorder {
            name: "first",
            order: order.clone(),
        });
        stack.push(OrderRecorder {
            name: "second",
            order: order.clone(),
        });
        let additions = stack.system_prompt_additions();
        assert_eq!(additions, vec!["[first]", "[second]"]);
    }
}

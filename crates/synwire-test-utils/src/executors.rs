//! Test directive executors.

use std::sync::{Arc, Mutex};
use synwire_core::BoxFuture;
use synwire_core::agents::directive::Directive;
use synwire_core::agents::directive_executor::{DirectiveError, DirectiveExecutor};

/// Recording executor that captures directives in a mutex for test assertions.
///
/// Example:
/// ```ignore
/// let executor = RecordingExecutor::new();
/// executor.execute_directive(&directive).await?;
/// let recorded = executor.recorded();
/// assert_eq!(recorded.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct RecordingExecutor {
    recorded: Arc<Mutex<Vec<Directive>>>,
}

impl RecordingExecutor {
    /// Create a new recording executor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            recorded: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all recorded directives.
    #[must_use]
    pub fn recorded(&self) -> Vec<Directive> {
        self.recorded
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Clear all recorded directives.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.recorded.lock() {
            guard.clear();
        }
    }
}

impl Default for RecordingExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectiveExecutor for RecordingExecutor {
    fn execute_directive(
        &self,
        directive: &Directive,
    ) -> BoxFuture<'_, Result<Option<serde_json::Value>, DirectiveError>> {
        let recorded = self.recorded.clone();
        let directive = directive.clone();
        Box::pin(async move {
            if let Ok(mut guard) = recorded.lock() {
                guard.push(directive);
            }
            Ok(None)
        })
    }
}

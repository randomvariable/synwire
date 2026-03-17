//! Output mode configuration for agents.

use serde::{Deserialize, Serialize};

/// System prompt configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SystemPromptConfig {
    /// Append to base system prompt.
    Append {
        /// Content to append.
        content: String,
    },
    /// Replace base system prompt.
    Replace {
        /// Replacement content.
        content: String,
    },
}

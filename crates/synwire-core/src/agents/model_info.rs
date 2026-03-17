//! Model information, capabilities, selection, and provider traits.

use serde::{Deserialize, Serialize};

use crate::BoxFuture;
use crate::agents::error::AgentError;

/// Model reasoning effort level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EffortLevel {
    /// Minimal reasoning.
    Low,
    /// Moderate reasoning.
    Medium,
    /// Deep reasoning (default).
    High,
    /// Maximum reasoning.
    Max,
}

/// Thinking/reasoning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ThinkingConfig {
    /// Model decides reasoning depth.
    Adaptive,
    /// Fixed token budget for reasoning.
    Enabled {
        /// Token budget for reasoning.
        budget_tokens: u32,
    },
    /// No reasoning/thinking.
    Disabled,
}

/// Model capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ModelCapabilities {
    /// Supports tool use.
    pub tool_calling: bool,
    /// Supports image input.
    pub vision: bool,
    /// Supports streaming output.
    pub streaming: bool,
    /// Supports native JSON mode.
    pub structured_output: bool,
    /// Supports reasoning effort levels.
    pub effort_levels: bool,
}

/// Model metadata and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier.
    pub id: String,
    /// Human-readable name.
    pub display_name: String,
    /// Model description.
    pub description: String,
    /// Feature support flags.
    pub capabilities: ModelCapabilities,
    /// Max context tokens.
    pub context_window: u32,
    /// Max output tokens.
    pub max_output_tokens: u32,
    /// Supported reasoning levels.
    pub supported_effort_levels: Vec<EffortLevel>,
}

// ---------------------------------------------------------------------------
// ModelProvider trait
// ---------------------------------------------------------------------------

/// Source of model metadata — implemented by each LLM provider crate.
pub trait ModelProvider: Send + Sync {
    /// Return all models offered by this provider.
    fn list_models(&self) -> BoxFuture<'_, Result<Vec<ModelInfo>, AgentError>>;
}

// ---------------------------------------------------------------------------
// ModelSelector
// ---------------------------------------------------------------------------

/// Queries a `ModelProvider` and selects models that meet specified criteria.
pub struct ModelSelector<'a> {
    models: &'a [ModelInfo],
}

impl<'a> ModelSelector<'a> {
    /// Create a selector over a slice of model infos.
    #[must_use]
    pub const fn new(models: &'a [ModelInfo]) -> Self {
        Self { models }
    }

    /// Find a model by exact ID.
    #[must_use]
    pub fn by_name(&self, id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Return all models from a provider whose ID starts with `prefix`.
    #[must_use]
    pub fn by_provider(&self, prefix: &str) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.id.starts_with(prefix))
            .collect()
    }

    /// Return all models that support tool calling.
    #[must_use]
    pub fn with_tool_calling(&self) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.capabilities.tool_calling)
            .collect()
    }

    /// Return all models that support vision (image input).
    #[must_use]
    pub fn with_vision(&self) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.capabilities.vision)
            .collect()
    }

    /// Return all models that support streaming.
    #[must_use]
    pub fn with_streaming(&self) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.capabilities.streaming)
            .collect()
    }

    /// Return all models that support native structured output.
    #[must_use]
    pub fn with_structured_output(&self) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.capabilities.structured_output)
            .collect()
    }

    /// Return all models that support effort levels.
    #[must_use]
    pub fn with_effort_levels(&self) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.capabilities.effort_levels)
            .collect()
    }

    /// Return all models with a context window at least `min_tokens`.
    #[must_use]
    pub fn by_min_context(&self, min_tokens: u32) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.context_window >= min_tokens)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model(id: &str, tool_calling: bool, vision: bool) -> ModelInfo {
        ModelInfo {
            id: id.to_string(),
            display_name: id.to_string(),
            description: String::new(),
            capabilities: ModelCapabilities {
                tool_calling,
                vision,
                streaming: true,
                structured_output: false,
                effort_levels: false,
            },
            context_window: 100_000,
            max_output_tokens: 4096,
            supported_effort_levels: Vec::new(),
        }
    }

    #[test]
    fn test_selector_by_name() {
        let models = vec![
            make_model("anthropic/claude-3-5-sonnet", true, true),
            make_model("openai/gpt-4o", true, false),
        ];
        let sel = ModelSelector::new(&models);
        assert!(sel.by_name("openai/gpt-4o").is_some());
        assert!(sel.by_name("nonexistent").is_none());
    }

    #[test]
    fn test_selector_by_provider() {
        let models = vec![
            make_model("anthropic/claude-3-5-sonnet", true, true),
            make_model("anthropic/claude-3-haiku", true, false),
            make_model("openai/gpt-4o", true, false),
        ];
        let sel = ModelSelector::new(&models);
        assert_eq!(sel.by_provider("anthropic/").len(), 2);
        assert_eq!(sel.by_provider("openai/").len(), 1);
    }

    #[test]
    fn test_selector_with_vision() {
        let models = vec![
            make_model("vision-model", true, true),
            make_model("text-only", true, false),
        ];
        let sel = ModelSelector::new(&models);
        let vision = sel.with_vision();
        assert_eq!(vision.len(), 1);
        assert_eq!(vision[0].id, "vision-model");
    }
}

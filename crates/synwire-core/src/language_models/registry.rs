//! Model profile registry for tracking model capabilities.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Information about a model's capabilities and metadata.
///
/// # Example
///
/// ```
/// use synwire_core::language_models::registry::ModelProfile;
///
/// let profile = ModelProfile {
///     model_id: "gpt-4o".into(),
///     provider: "openai".into(),
///     supports_tools: true,
///     supports_streaming: true,
///     supports_structured_output: true,
///     max_context_tokens: Some(128_000),
///     max_output_tokens: Some(16_384),
/// };
///
/// assert!(profile.supports_tools);
/// ```
#[derive(Debug, Clone)]
pub struct ModelProfile {
    /// Model identifier (e.g., "gpt-4o", "claude-3-opus").
    pub model_id: String,
    /// Provider name (e.g., "openai", "anthropic").
    pub provider: String,
    /// Whether the model supports tool calling.
    pub supports_tools: bool,
    /// Whether the model supports streaming.
    pub supports_streaming: bool,
    /// Whether the model supports structured output.
    pub supports_structured_output: bool,
    /// Maximum context window size in tokens.
    pub max_context_tokens: Option<u64>,
    /// Maximum output tokens.
    pub max_output_tokens: Option<u64>,
}

/// Well-known capability names for [`ModelProfileRegistry::supports`].
///
/// These constants match the string values accepted by the `supports` method.
pub mod capabilities {
    /// Tool calling capability.
    pub const TOOLS: &str = "tools";
    /// Streaming capability.
    pub const STREAMING: &str = "streaming";
    /// Structured output capability.
    pub const STRUCTURED_OUTPUT: &str = "structured_output";
}

/// Registry for model profiles.
///
/// Provides capability look-ups by model identifier.
pub trait ModelProfileRegistry: Send + Sync {
    /// Register a model profile.
    ///
    /// If a profile with the same `model_id` already exists, it is replaced.
    fn register(&self, profile: ModelProfile);

    /// Get a model profile by model ID.
    fn get(&self, model_id: &str) -> Option<ModelProfile>;

    /// Check if a model supports a specific capability.
    ///
    /// Known capability strings: `"tools"`, `"streaming"`, `"structured_output"`.
    /// Unknown capabilities return `false`.
    fn supports(&self, model_id: &str, capability: &str) -> bool;
}

/// In-memory implementation of [`ModelProfileRegistry`].
///
/// Thread-safe via an internal `RwLock`. Cloning shares the same backing store.
///
/// # Example
///
/// ```
/// use synwire_core::language_models::registry::{
///     InMemoryModelProfileRegistry, ModelProfile, ModelProfileRegistry,
/// };
///
/// let registry = InMemoryModelProfileRegistry::default();
/// registry.register(ModelProfile {
///     model_id: "gpt-4o".into(),
///     provider: "openai".into(),
///     supports_tools: true,
///     supports_streaming: true,
///     supports_structured_output: false,
///     max_context_tokens: Some(128_000),
///     max_output_tokens: Some(16_384),
/// });
///
/// assert!(registry.supports("gpt-4o", "tools"));
/// assert!(!registry.supports("gpt-4o", "structured_output"));
/// assert!(registry.get("gpt-4o").is_some());
/// ```
#[derive(Debug, Clone, Default)]
pub struct InMemoryModelProfileRegistry {
    profiles: Arc<RwLock<HashMap<String, ModelProfile>>>,
}

impl InMemoryModelProfileRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ModelProfileRegistry for InMemoryModelProfileRegistry {
    fn register(&self, profile: ModelProfile) {
        let Ok(mut map) = self.profiles.write() else {
            return;
        };
        let _ = map.insert(profile.model_id.clone(), profile);
    }

    fn get(&self, model_id: &str) -> Option<ModelProfile> {
        let map = self.profiles.read().ok()?;
        map.get(model_id).cloned()
    }

    fn supports(&self, model_id: &str, capability: &str) -> bool {
        let Some(profile) = self.get(model_id) else {
            return false;
        };
        match capability {
            capabilities::TOOLS => profile.supports_tools,
            capabilities::STREAMING => profile.supports_streaming,
            capabilities::STRUCTURED_OUTPUT => profile.supports_structured_output,
            _ => false,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sample_profile() -> ModelProfile {
        ModelProfile {
            model_id: "gpt-4o".into(),
            provider: "openai".into(),
            supports_tools: true,
            supports_streaming: true,
            supports_structured_output: false,
            max_context_tokens: Some(128_000),
            max_output_tokens: Some(16_384),
        }
    }

    #[test]
    fn register_and_retrieve() {
        let registry = InMemoryModelProfileRegistry::new();
        registry.register(sample_profile());

        let profile = registry.get("gpt-4o").unwrap();
        assert_eq!(profile.model_id, "gpt-4o");
        assert_eq!(profile.provider, "openai");
        assert!(profile.supports_tools);
        assert_eq!(profile.max_context_tokens, Some(128_000));
    }

    #[test]
    fn get_returns_none_for_unknown_model() {
        let registry = InMemoryModelProfileRegistry::new();
        assert!(registry.get("nonexistent-model").is_none());
    }

    #[test]
    fn supports_tools() {
        let registry = InMemoryModelProfileRegistry::new();
        registry.register(sample_profile());

        assert!(registry.supports("gpt-4o", "tools"));
    }

    #[test]
    fn supports_streaming() {
        let registry = InMemoryModelProfileRegistry::new();
        registry.register(sample_profile());

        assert!(registry.supports("gpt-4o", "streaming"));
    }

    #[test]
    fn supports_structured_output_false() {
        let registry = InMemoryModelProfileRegistry::new();
        registry.register(sample_profile());

        assert!(!registry.supports("gpt-4o", "structured_output"));
    }

    #[test]
    fn supports_unknown_capability() {
        let registry = InMemoryModelProfileRegistry::new();
        registry.register(sample_profile());

        assert!(!registry.supports("gpt-4o", "vision"));
    }

    #[test]
    fn supports_unknown_model() {
        let registry = InMemoryModelProfileRegistry::new();
        assert!(!registry.supports("nonexistent", "tools"));
    }

    #[test]
    fn register_replaces_existing() {
        let registry = InMemoryModelProfileRegistry::new();
        registry.register(sample_profile());

        let mut updated = sample_profile();
        updated.supports_structured_output = true;
        registry.register(updated);

        assert!(registry.supports("gpt-4o", "structured_output"));
    }

    #[test]
    fn clone_shares_state() {
        let registry = InMemoryModelProfileRegistry::new();
        let cloned = registry.clone();

        registry.register(sample_profile());
        assert!(cloned.get("gpt-4o").is_some());
    }
}

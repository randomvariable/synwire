//! Output mode selection for structured output.

use crate::error::SynwireError;

/// Strategy for extracting structured output from a model.
///
/// Different providers support different mechanisms for structured output.
/// This enum allows callers to specify their preferred extraction strategy,
/// and [`validate_compatibility`](OutputMode::validate_compatibility) can
/// verify that the chosen mode is supported.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputMode {
    /// Use the model's native structured output support (e.g., `response_format`).
    Native,
    /// Use tool calling to extract structured output.
    Tool,
    /// Include format instructions in the prompt.
    Prompt,
    /// Custom extraction strategy.
    Custom(String),
}

impl OutputMode {
    /// Returns the fallback chain order for output modes.
    ///
    /// The chain tries the most capable mode first (native structured output),
    /// then tool calling, and finally prompt-based instructions as a universal
    /// fallback.
    pub fn fallback_chain() -> Vec<Self> {
        vec![Self::Native, Self::Tool, Self::Prompt]
    }

    /// Validate that a mode is compatible with a provider's capabilities.
    ///
    /// # Errors
    ///
    /// Returns [`SynwireError::Prompt`] if the mode requires a capability that
    /// the provider does not support.
    pub fn validate_compatibility(
        &self,
        supports_native: bool,
        supports_tools: bool,
    ) -> Result<(), SynwireError> {
        match self {
            Self::Native if !supports_native => Err(SynwireError::Prompt {
                message: "Provider does not support native structured output".into(),
            }),
            Self::Tool if !supports_tools => Err(SynwireError::Prompt {
                message: "Provider does not support tool calling".into(),
            }),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_chain_order() {
        let chain = OutputMode::fallback_chain();
        assert_eq!(
            chain,
            vec![OutputMode::Native, OutputMode::Tool, OutputMode::Prompt]
        );
    }

    #[test]
    fn test_native_rejects_unsupported() {
        let result = OutputMode::Native.validate_compatibility(false, true);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("native structured output"));
    }

    #[test]
    fn test_tool_rejects_unsupported() {
        let result = OutputMode::Tool.validate_compatibility(true, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("tool calling"));
    }

    #[test]
    fn test_prompt_always_compatible() {
        let result = OutputMode::Prompt.validate_compatibility(false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_native_accepts_supported() {
        let result = OutputMode::Native.validate_compatibility(true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_accepts_supported() {
        let result = OutputMode::Tool.validate_compatibility(false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_always_compatible() {
        let result = OutputMode::Custom("my_strategy".into()).validate_compatibility(false, false);
        assert!(result.is_ok());
    }
}

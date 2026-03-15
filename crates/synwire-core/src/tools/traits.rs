//! Tool trait and name validation.

use crate::BoxFuture;
use crate::error::{SynwireError, ToolError};
use crate::tools::types::{ToolOutput, ToolSchema};

/// Trait for callable tools.
///
/// Implementations must be `Send + Sync` so tools can be shared across
/// async tasks and threads.
///
/// # Cancel safety
///
/// The future returned by [`invoke`](Self::invoke) is **not cancel-safe**
/// in general. If the tool performs side effects (file writes, API calls),
/// dropping the future mid-execution may leave those effects partially
/// applied. Callers should avoid dropping tool futures unless they are
/// prepared to retry or roll back.
///
/// # Example
///
/// ```
/// use synwire_core::tools::{Tool, ToolOutput, ToolSchema};
/// use synwire_core::error::SynwireError;
/// use synwire_core::BoxFuture;
///
/// struct Echo;
///
/// impl Tool for Echo {
///     fn name(&self) -> &str { "echo" }
///     fn description(&self) -> &str { "Echoes input back" }
///     fn schema(&self) -> &ToolSchema {
///         // In real code, store this in a field
///         Box::leak(Box::new(ToolSchema {
///             name: "echo".into(),
///             description: "Echoes input back".into(),
///             parameters: serde_json::json!({"type": "object"}),
///         }))
///     }
///     fn invoke(
///         &self,
///         input: serde_json::Value,
///     ) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
///         Box::pin(async move {
///             Ok(ToolOutput {
///                 content: input.to_string(),
///                 artifact: None,
///             })
///         })
///     }
/// }
/// ```
pub trait Tool: Send + Sync {
    /// The tool's name.
    fn name(&self) -> &str;

    /// The tool's description.
    fn description(&self) -> &str;

    /// The tool's schema for argument validation.
    fn schema(&self) -> &ToolSchema;

    /// Invoke the tool with JSON arguments.
    fn invoke(&self, input: serde_json::Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>>;
}

/// Validate a tool name against the pattern `^[a-zA-Z0-9_-]{1,64}$`.
///
/// # Errors
///
/// Returns [`SynwireError::Tool`] with [`ToolError::InvalidName`] if the name
/// is empty, longer than 64 characters, or contains disallowed characters.
pub fn validate_tool_name(name: &str) -> Result<(), SynwireError> {
    if name.is_empty() || name.len() > 64 {
        return Err(SynwireError::Tool(ToolError::InvalidName {
            name: name.into(),
            reason: "name must be 1-64 characters".into(),
        }));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(SynwireError::Tool(ToolError::InvalidName {
            name: name.into(),
            reason: "name must match [a-zA-Z0-9_-]".into(),
        }));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        validate_tool_name("search").unwrap();
        validate_tool_name("my-tool").unwrap();
        validate_tool_name("tool_123").unwrap();
        validate_tool_name("A").unwrap();
        // 64-char name
        let long_name = "a".repeat(64);
        validate_tool_name(&long_name).unwrap();
    }

    #[test]
    fn rejects_empty_name() {
        let err = validate_tool_name("").unwrap_err();
        assert!(err.to_string().contains("1-64 characters"));
    }

    #[test]
    fn rejects_too_long_name() {
        let name = "a".repeat(65);
        let err = validate_tool_name(&name).unwrap_err();
        assert!(err.to_string().contains("1-64 characters"));
    }

    #[test]
    fn rejects_special_characters() {
        let err = validate_tool_name("my tool").unwrap_err();
        assert!(err.to_string().contains("[a-zA-Z0-9_-]"));
    }

    #[test]
    fn rejects_dots() {
        let err = validate_tool_name("my.tool").unwrap_err();
        assert!(err.to_string().contains("[a-zA-Z0-9_-]"));
    }
}

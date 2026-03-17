//! Permission modes and rules.

use serde::{Deserialize, Serialize};

/// Permission mode presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum PermissionMode {
    /// Prompt for dangerous operations (default).
    #[default]
    Default,
    /// Auto-approve file modifications.
    AcceptEdits,
    /// Read-only, no mutations.
    PlanOnly,
    /// Auto-approve everything.
    BypassAll,
    /// Deny if no pre-approved rule matches.
    DenyUnauthorized,
}

/// Permission behavior for a tool pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PermissionBehavior {
    /// Allow the operation.
    Allow,
    /// Deny the operation.
    Deny,
    /// Ask the user.
    Ask,
}

/// Declarative permission rule for tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Glob pattern for tool names.
    pub tool_pattern: String,
    /// Behavior when pattern matches.
    pub behavior: PermissionBehavior,
}

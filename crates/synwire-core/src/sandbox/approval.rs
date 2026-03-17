//! Approval gates for risky operations.

use crate::BoxFuture;
use serde::{Deserialize, Serialize};

/// Risk classification for an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RiskLevel {
    /// No meaningful risk (read-only).
    None,
    /// Low risk (reversible writes).
    Low,
    /// Medium risk (file deletions, overwrites).
    Medium,
    /// High risk (system changes, process spawning).
    High,
    /// Critical risk (irreversible, destructive).
    Critical,
}

/// Request for human or automated approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Operation identifier.
    pub operation: String,
    /// Human-readable description of what will happen.
    pub description: String,
    /// Risk classification.
    pub risk: RiskLevel,
    /// Optional timeout in seconds (None = no timeout).
    pub timeout_secs: Option<u64>,
    /// Arguments or context for the operation.
    pub context: serde_json::Value,
}

/// Decision returned by an approval gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ApprovalDecision {
    /// Allow this invocation.
    Allow,
    /// Deny this invocation.
    Deny,
    /// Allow this invocation and all future invocations of the same operation.
    AllowAlways,
    /// Abort the entire agent run.
    Abort,
    /// Allow but with modified input.
    AllowModified {
        /// Modified context to use instead.
        modified_context: serde_json::Value,
    },
}

/// Trait for approval gate callbacks.
///
/// Implementors decide whether a risky operation should proceed.
pub trait ApprovalCallback: Send + Sync {
    /// Evaluate an approval request and return a decision.
    fn request(&self, req: ApprovalRequest) -> BoxFuture<'_, ApprovalDecision>;
}

/// Approval callback that auto-approves everything.
#[derive(Debug, Clone, Default)]
pub struct AutoApproveCallback;

impl ApprovalCallback for AutoApproveCallback {
    fn request(&self, _req: ApprovalRequest) -> BoxFuture<'_, ApprovalDecision> {
        Box::pin(async { ApprovalDecision::Allow })
    }
}

/// Approval callback that auto-denies everything.
#[derive(Debug, Clone, Default)]
pub struct AutoDenyCallback;

impl ApprovalCallback for AutoDenyCallback {
    fn request(&self, _req: ApprovalRequest) -> BoxFuture<'_, ApprovalDecision> {
        Box::pin(async { ApprovalDecision::Deny })
    }
}

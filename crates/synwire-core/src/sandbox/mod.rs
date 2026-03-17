//! Sandbox protocols for command execution and approval gates.
//!
//! Separated from the VFS module because command execution, process management,
//! and archive manipulation are distinct concerns from filesystem abstraction.

pub mod approval;

use crate::BoxFuture;
use crate::vfs::error::VfsError;
use crate::vfs::types::ExecuteResponse;
use serde::{Deserialize, Serialize};

pub use approval::{
    ApprovalCallback, ApprovalDecision, ApprovalRequest, AutoApproveCallback, AutoDenyCallback,
    RiskLevel,
};

/// A single stage in a command pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    /// Command to execute.
    pub command: String,
    /// Arguments.
    pub args: Vec<String>,
    /// Redirect stderr to stdout.
    pub stderr_to_stdout: bool,
    /// Per-stage timeout in seconds (None = no limit).
    pub timeout_secs: Option<u64>,
}

/// Sandbox protocol for command execution with isolation.
///
/// Separate from [`Vfs`](crate::vfs::protocol::Vfs) to make it
/// explicit when an agent needs shell execution capability.
pub trait SandboxProtocol: Send + Sync {
    /// Execute a single command.
    fn execute(
        &self,
        cmd: &str,
        args: &[String],
    ) -> BoxFuture<'_, Result<ExecuteResponse, VfsError>>;

    /// Execute a multi-stage pipeline (each stage's stdout pipes into the next).
    fn execute_pipeline(
        &self,
        stages: &[PipelineStage],
    ) -> BoxFuture<'_, Result<Vec<ExecuteResponse>, VfsError>>;

    /// Sandbox identifier (for logging / audit).
    fn id(&self) -> &str;
}

/// Abstract sandbox type returned by sandbox factory functions.
pub type BaseSandbox = dyn SandboxProtocol + Send + Sync;

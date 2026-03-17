//! Sandbox error type.

use thiserror::Error;

/// Errors produced by sandbox operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SandboxError {
    /// cgroup v2 is not available on this system (non-systemd or pre-unified
    /// hierarchy kernel).
    #[error("cgroup v2 not available: {0}")]
    CgroupUnavailable(String),

    /// A filesystem I/O error occurred while manipulating cgroupfs.
    #[error("cgroup I/O error: {0}")]
    CgroupIo(#[from] std::io::Error),

    /// The discovered cgroup path is not writable by the current user.
    #[error("cgroup path not writable: {path}")]
    CgroupNotWritable {
        /// The cgroup path that was found but could not be written to.
        path: String,
    },

    /// Failed to parse `/proc/self/cgroup`.
    #[error("failed to parse /proc/self/cgroup: {0}")]
    CgroupParseFailed(String),

    /// A process registry limit was exceeded.
    #[error("process registry full: max_tracked={max_tracked}")]
    RegistryFull {
        /// The configured limit.
        max_tracked: usize,
    },

    /// The requested PID was not found in the registry.
    #[error("process not found: pid={pid}")]
    ProcessNotFound {
        /// The process ID that was not found.
        pid: u32,
    },

    /// No OCI container runtime could be found on `$PATH`.
    #[error("OCI runtime '{name}' not found on $PATH")]
    RuntimeNotFound {
        /// Name of the binary that was searched for.
        name: String,
    },

    /// The container runtime exited with a non-zero status or failed to start.
    #[error("container runtime failed: {reason}")]
    RuntimeFailed {
        /// Human-readable reason.
        reason: String,
    },

    /// A signal could not be sent to the target process.
    #[error("failed to send signal to pid={pid}: {reason}")]
    SignalFailed {
        /// Target process ID.
        pid: u32,
        /// OS error message.
        reason: String,
    },

    /// An approval callback denied the operation.
    #[error("operation denied by approval callback: {operation}")]
    ApprovalDenied {
        /// The operation that was denied.
        operation: String,
    },

    /// Serialization or deserialization error when communicating with the
    /// init binary.
    #[error("sandbox protocol serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// Generic platform error for unsupported operations.
    #[error("operation not supported on this platform: {0}")]
    Unsupported(String),
}

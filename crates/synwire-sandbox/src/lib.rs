//! Platform-specific sandbox backends for synwire agents.
//!
//! This crate provides process isolation, resource accounting, and
//! LLM-accessible process management tools. Namespace isolation is provided
//! by an OCI runtime (runc/crun) — no custom init binary needed.
//!
//! # Safety
//!
//! This crate uses `#![deny(unsafe_code)]` with a single scoped exception:
//! receiving a PTY controller fd from the OCI runtime via `SCM_RIGHTS`
//! requires converting a kernel-provided raw fd to an `OwnedFd`.
//!
//! # Platform support
//!
//! | Platform | Light isolation | Strong isolation |
//! |----------|----------------|-----------------|
//! | Linux    | cgroup v2 + AppArmor | Namespace container |
//! | macOS    | `sandbox-exec` Seatbelt | Podman / Lima |
//! | Other    | None (fallback) | None |

#![deny(unsafe_code)]

pub mod error;
pub mod output;
pub mod platform;
pub mod plugin;
pub mod process_registry;
pub mod visibility;

pub use error::SandboxError;
pub use output::{CapturedOutput, OutputMode, ProcessCapture};
pub use process_registry::{ProcessRecord, ProcessRegistry, ProcessStatus};
pub use visibility::ProcessVisibilityScope;

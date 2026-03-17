//! Shared context for sandbox tools that need to spawn processes.
//!
//! [`SandboxContext`] bundles the OCI runtime, sandbox configuration, and
//! process registry into a single shareable handle. Tools that spawn or
//! interact with sandboxed processes (e.g., `run_command`, `open_shell`)
//! hold an `Arc<SandboxContext>`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use synwire_core::agents::sandbox::SandboxConfig;

use crate::process_registry::ProcessRegistry;
use crate::visibility::ProcessVisibilityScope;

use super::expect_engine::{PtyStream, StubProcess};

#[cfg(target_os = "linux")]
use crate::platform::linux::namespace::NamespaceContainer;

/// An expectrl session wrapping a PTY controller fd from the OCI runtime.
pub type ExpectSession = expectrl::Session<StubProcess, PtyStream>;

/// Shared context for sandbox LLM tools.
///
/// Holds everything needed to spawn sandboxed processes and PTY sessions.
/// Passed as `Arc<SandboxContext>` to tools like `run_command` and `open_shell`.
#[derive(Debug)]
pub struct SandboxContext {
    /// The sandbox configuration (filesystem rules, security, resources).
    pub config: SandboxConfig,
    /// The process registry for this agent.
    pub registry: Arc<RwLock<ProcessRegistry>>,
    /// Visibility scope (own + child registries).
    pub scope: ProcessVisibilityScope,
    /// The OCI runtime container (Linux only).
    #[cfg(target_os = "linux")]
    pub container: NamespaceContainer,
    /// Active expectrl sessions keyed by session ID.
    /// Each session wraps a PTY controller fd and provides expect/send/batch.
    pub(crate) sessions: Mutex<HashMap<String, ExpectSession>>,
    /// OCI runtime child processes for active shell sessions, keyed by session ID.
    /// Kept alive alongside the session — dropped when the session is closed.
    #[cfg(target_os = "linux")]
    pub(crate) session_children: Mutex<HashMap<String, tokio::process::Child>>,
}

impl SandboxContext {
    /// Create a new sandbox context.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::SandboxError`] if the OCI runtime cannot be found.
    #[cfg(target_os = "linux")]
    pub fn new(
        config: SandboxConfig,
        registry: Arc<RwLock<ProcessRegistry>>,
        scope: ProcessVisibilityScope,
        container: NamespaceContainer,
    ) -> Self {
        Self {
            config,
            registry,
            scope,
            container,
            sessions: Mutex::new(HashMap::new()),
            session_children: Mutex::new(HashMap::new()),
        }
    }
}

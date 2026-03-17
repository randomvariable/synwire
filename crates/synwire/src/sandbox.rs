//! Sandbox integration for agent tool calling.
//!
//! Provides `SandboxedAgent::with_sandbox()` which wires up the full
//! sandbox tool suite automatically:
//!
//! - **Management tools**: `list_processes`, `kill_process`, `process_stats`,
//!   `wait_for_process`, `read_process_output`
//! - **Command tools** (Linux only): `run_command`, `open_shell`,
//!   `shell_write`, `shell_read`
//!
//! # Example
//!
//! ```rust,ignore
//! use synwire::agent::prelude::*;
//! use synwire::sandbox::SandboxedAgent;
//! use synwire_core::agents::sandbox::SandboxConfig;
//!
//! let (agent, handle) = Agent::<()>::new("my-agent", "gpt-4")
//!     .with_sandbox(SandboxConfig::default());
//!
//! // The LLM can now call:
//! //   run_command({"command": "ls", "args": ["-la"]})
//! //   open_shell({})
//! //   shell_write({"session_id": "...", "input": "echo hi\n"})
//! //   shell_read({"session_id": "..."})
//! //   list_processes({})
//! //   wait_for_process({"pid": 42})
//! //   read_process_output({"pid": 42, "stream": "stdout"})
//! //   kill_process({"pid": 42})
//! //   process_stats({"pid": 42})
//! ```

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::sandbox::SandboxConfig;
use synwire_sandbox::plugin::process_plugin::ProcessPlugin;
use synwire_sandbox::process_registry::ProcessRegistry;
use synwire_sandbox::visibility::ProcessVisibilityScope;

/// Re-exports for sandbox types.
pub use synwire_sandbox::{
    CapturedOutput, OutputMode, ProcessCapture, ProcessRecord, ProcessStatus,
};

/// Handle returned by [`SandboxedAgent::with_sandbox`], providing access to
/// the process registry and visibility scope for the agent.
///
/// Use [`SandboxHandle::scope`] to register child agent registries for
/// parent-child process visibility.
#[derive(Debug, Clone)]
pub struct SandboxHandle {
    /// The process registry backing this agent's process tracking.
    pub registry: Arc<RwLock<ProcessRegistry>>,
    /// Visibility scope — add child registries via
    /// [`scope.add_child_registry()`](ProcessVisibilityScope::add_child_registry).
    pub scope: ProcessVisibilityScope,
}

/// Extension trait that adds `.with_sandbox()` to the `Agent` builder.
pub trait SandboxedAgent<O: Serialize + Send + Sync + 'static> {
    /// Configure sandboxing with automatic plugin wiring.
    ///
    /// On Linux, locates an OCI runtime (`runc`) and creates a
    /// [`SandboxContext`](synwire_sandbox::plugin::SandboxContext) with the
    /// full command execution tool suite (`run_command`, `open_shell`,
    /// `shell_write`, `shell_read`).
    ///
    /// On non-Linux platforms, or if no OCI runtime is found, falls back to
    /// management-only tools (`list_processes`, `kill_process`, etc.).
    ///
    /// Returns `(agent_builder, handle)` — the handle provides access to the
    /// registry and scope for wiring up sub-agent visibility.
    fn with_sandbox(self, config: SandboxConfig) -> (Self, SandboxHandle)
    where
        Self: Sized;

    /// Configure sandboxing with a custom max-process limit.
    fn with_sandbox_limit(self, config: SandboxConfig, max_tracked: usize) -> (Self, SandboxHandle)
    where
        Self: Sized;
}

impl<O: Serialize + Send + Sync + 'static> SandboxedAgent<O> for Agent<O> {
    fn with_sandbox(self, config: SandboxConfig) -> (Self, SandboxHandle) {
        self.with_sandbox_limit(config, 256)
    }

    fn with_sandbox_limit(
        self,
        config: SandboxConfig,
        max_tracked: usize,
    ) -> (Self, SandboxHandle) {
        let registry = Arc::new(RwLock::new(ProcessRegistry::new(Some(max_tracked))));
        let scope = ProcessVisibilityScope::new(Arc::clone(&registry));

        let handle = SandboxHandle {
            registry: Arc::clone(&registry),
            scope: scope.clone(),
        };

        // Try to create full command tools (requires OCI runtime on Linux).
        let plugin = try_full_plugin(config.clone(), registry, scope);

        let agent = self.plugin(plugin).sandbox(config);
        (agent, handle)
    }
}

/// Try to create a full `ProcessPlugin` with command execution tools.
/// Falls back to management-only tools if the OCI runtime isn't available.
fn try_full_plugin(
    config: SandboxConfig,
    registry: Arc<RwLock<ProcessRegistry>>,
    scope: ProcessVisibilityScope,
) -> ProcessPlugin {
    #[cfg(target_os = "linux")]
    {
        use synwire_sandbox::platform::linux::namespace::NamespaceContainer;
        use synwire_sandbox::plugin::context::SandboxContext;

        match NamespaceContainer::new() {
            Ok(container) => {
                let ctx = Arc::new(SandboxContext::new(config, registry, scope, container));
                return ProcessPlugin::with_context(ctx);
            }
            Err(e) => {
                tracing::warn!(
                    "OCI runtime not found ({e}), sandbox tools will be management-only \
                     (no run_command/open_shell)"
                );
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    let _ = config;

    ProcessPlugin::with_scope(scope)
}

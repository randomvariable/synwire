//! `ProcessPlugin` — exposes process management and command execution as LLM tools.
//!
//! Implements the synwire-core [`Plugin`] trait. Contributes two sets of tools:
//!
//! **Management tools** (always available):
//! `list_processes`, `kill_process`, `process_stats`, `wait_for_process`,
//! `read_process_output`.
//!
//! **Command tools** (when [`SandboxContext`] is provided):
//! `run_command`, `open_shell`, `shell_write`, `shell_read`.
//!
//! # Parent-child visibility
//!
//! Use [`ProcessVisibilityScope::add_child_registry`] to grant a parent agent
//! read access to a sub-agent's processes. Read tools (`list`, `stats`,
//! `wait`, `read_output`) see all visible registries; write tools (`kill`)
//! are restricted to the agent's own registry.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use synwire_core::agents::plugin::{Plugin, PluginStateKey};
use synwire_core::tools::Tool;

use crate::plugin::command_tools::{
    OpenShellTool, RunCommandTool, ShellBatchTool, ShellExpectCasesTool, ShellExpectTool,
    ShellReadTool, ShellSignalTool, ShellWriteTool,
};
use crate::plugin::context::SandboxContext;
use crate::plugin::tools::{
    KillProcessTool, ListProcessesTool, ProcessStatsTool, ReadProcessOutputTool, WaitForProcessTool,
};
use crate::process_registry::ProcessRegistry;
use crate::visibility::ProcessVisibilityScope;

// ── Plugin state key ──────────────────────────────────────────────────────────

/// Shared state owned by `ProcessPlugin`.
#[derive(Debug)]
pub struct ProcessPluginState {
    /// Thread-safe process registry.
    pub registry: Arc<RwLock<ProcessRegistry>>,
}

// Minimal serialization for `PluginStateMap::serialize_all` (returns registry size).
impl Serialize for ProcessPluginState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("active", &true)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for ProcessPluginState {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        // Deserialization creates an empty registry — full state cannot be
        // reconstructed from JSON (PIDs are ephemeral).
        Ok(Self {
            registry: Arc::new(RwLock::new(ProcessRegistry::new(None))),
        })
    }
}

/// [`PluginStateKey`] for `ProcessPlugin`.
pub struct ProcessPluginKey;

impl PluginStateKey for ProcessPluginKey {
    type State = ProcessPluginState;
    const KEY: &'static str = "synwire.process";
}

// ── ProcessPlugin ─────────────────────────────────────────────────────────────

/// Plugin that tracks spawned processes and provides LLM tool access.
///
/// # Management-only (no command execution)
///
/// ```rust,ignore
/// let plugin = ProcessPlugin::with_scope(scope);
/// // Provides: list_processes, kill_process, process_stats,
/// //           wait_for_process, read_process_output
/// ```
///
/// # Full command execution
///
/// ```rust,ignore
/// let ctx = Arc::new(SandboxContext::new(config, registry, scope, container));
/// let plugin = ProcessPlugin::with_context(ctx);
/// // Provides all management tools PLUS:
/// //   run_command, open_shell, shell_write, shell_read
/// ```
pub struct ProcessPlugin {
    tools: Vec<Arc<dyn Tool>>,
}

impl ProcessPlugin {
    /// Create a plugin with management tools only.
    ///
    /// Convenience constructor that wraps the registry in a
    /// [`ProcessVisibilityScope`] with no child registries.
    #[must_use]
    pub fn new(registry: Arc<RwLock<ProcessRegistry>>) -> Self {
        Self::with_scope(ProcessVisibilityScope::new(registry))
    }

    /// Create a plugin with management tools backed by a visibility scope.
    ///
    /// Use this when the agent has sub-agents whose process registries should
    /// be visible to read-only tools (`list`, `stats`, `wait`, `read_output`).
    #[must_use]
    pub fn with_scope(scope: ProcessVisibilityScope) -> Self {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ListProcessesTool::new(scope.clone())),
            Arc::new(KillProcessTool::new(scope.clone())),
            Arc::new(ProcessStatsTool::new(scope.clone())),
            Arc::new(WaitForProcessTool::new(scope.clone())),
            Arc::new(ReadProcessOutputTool::new(scope)),
        ];
        Self { tools }
    }

    /// Create a plugin with **all** tools: management + command execution.
    ///
    /// The [`SandboxContext`] provides the OCI runtime, sandbox config, and
    /// process registry needed by `run_command`, `open_shell`, `shell_write`,
    /// and `shell_read`.
    #[must_use]
    pub fn with_context(ctx: Arc<SandboxContext>) -> Self {
        let scope = ctx.scope.clone();
        let mut tools: Vec<Arc<dyn Tool>> = vec![
            // Management tools
            Arc::new(ListProcessesTool::new(scope.clone())),
            Arc::new(KillProcessTool::new(scope.clone())),
            Arc::new(ProcessStatsTool::new(scope.clone())),
            Arc::new(WaitForProcessTool::new(scope.clone())),
            Arc::new(ReadProcessOutputTool::new(scope)),
            // Command execution tools
            Arc::new(RunCommandTool::new(Arc::clone(&ctx))),
            Arc::new(OpenShellTool::new(Arc::clone(&ctx))),
            Arc::new(ShellWriteTool::new(Arc::clone(&ctx))),
            Arc::new(ShellReadTool::new(Arc::clone(&ctx))),
            Arc::new(ShellExpectTool::new(Arc::clone(&ctx))),
            Arc::new(ShellExpectCasesTool::new(Arc::clone(&ctx))),
            Arc::new(ShellBatchTool::new(Arc::clone(&ctx))),
            Arc::new(ShellSignalTool::new(ctx)),
        ];
        // Sort for deterministic tool ordering in schema.
        tools.sort_by(|a, b| a.name().cmp(b.name()));
        Self { tools }
    }
}

impl Plugin for ProcessPlugin {
    fn name(&self) -> &'static str {
        "synwire-process"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}

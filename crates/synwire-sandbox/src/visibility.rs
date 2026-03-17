#![allow(clippy::type_complexity, clippy::significant_drop_tightening)]
//! Parent-child process visibility scoping.
//!
//! [`ProcessVisibilityScope`] controls which process registries an agent can
//! read.  Parent agents see their own processes **and** all sub-agent
//! processes; child agents see only their own.
//!
//! Write operations (signal, kill) always target the agent's own registry —
//! a child agent cannot kill a parent's processes.

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::process_registry::{ProcessRecord, ProcessRegistry};

/// Scoped view of process registries for parent-child visibility.
///
/// # Visibility rules
///
/// | Operation | Own processes | Child processes |
/// |-----------|:------------:|:---------------:|
/// | list      | yes          | yes             |
/// | stats     | yes          | yes (read-only) |
/// | wait      | yes          | yes             |
/// | read output | yes        | yes             |
/// | kill      | yes          | **no**          |
#[derive(Debug, Clone)]
pub struct ProcessVisibilityScope {
    /// This agent's own process registry.
    pub own: Arc<RwLock<ProcessRegistry>>,
    /// Child agent registries visible to this agent.
    children: Arc<tokio::sync::Mutex<Vec<(String, Arc<RwLock<ProcessRegistry>>)>>>,
}

impl ProcessVisibilityScope {
    /// Create a new scope backed by the given registry (no children initially).
    #[must_use]
    pub fn new(registry: Arc<RwLock<ProcessRegistry>>) -> Self {
        Self {
            own: registry,
            children: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    /// Register a child agent's registry as visible to this agent.
    ///
    /// `label` is a display name (e.g., agent UUID or name) used to tag
    /// child processes in listing output.
    pub async fn add_child_registry(
        &self,
        label: impl Into<String>,
        registry: Arc<RwLock<ProcessRegistry>>,
    ) {
        self.children.lock().await.push((label.into(), registry));
    }

    /// Collect all running processes visible to this agent.
    ///
    /// Returns `(agent_label, record)` pairs.  `agent_label` is `None` for
    /// own processes and `Some(label)` for child-agent processes.
    pub async fn visible_running(&self) -> Vec<(Option<String>, ProcessRecord)> {
        let mut result = Vec::new();

        {
            let own = self.own.read().await;
            for r in own.running() {
                result.push((None, r.clone()));
            }
        }

        let children = self.children.lock().await;
        for (label, reg) in children.iter() {
            let child_reg = reg.read().await;
            for r in child_reg.running() {
                result.push((Some(label.clone()), r.clone()));
            }
        }

        result
    }

    /// Look up a process in all visible registries (own first, then children).
    ///
    /// Returns `(agent_label, record)`.  Returns `None` if the PID is not
    /// found in any visible registry.
    pub async fn find(&self, pid: u32) -> Option<(Option<String>, ProcessRecord)> {
        {
            let own = self.own.read().await;
            if let Some(r) = own.get(pid) {
                return Some((None, r.clone()));
            }
        }

        let children = self.children.lock().await;
        for (label, reg) in children.iter() {
            let child_reg = reg.read().await;
            if let Some(r) = child_reg.get(pid) {
                return Some((Some(label.clone()), r.clone()));
            }
        }

        None
    }
}

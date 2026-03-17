//! In-memory registry of processes spawned by sandbox backends.
//!
//! Each agent that enables process tracking holds an
//! `Arc<RwLock<ProcessRegistry>>`. The registry is updated by sandbox backends
//! when processes are spawned or exit, and read by the LLM tools
//! (`list_processes`, `kill_process`, `process_stats`, `wait_for_process`,
//! `read_process_output`).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::SandboxError;
use crate::output::CapturedOutput;

// ── ProcessStatus ─────────────────────────────────────────────────────────────

/// Observed lifecycle state of a tracked process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProcessStatus {
    /// Process is running.
    Running,
    /// Process exited normally.
    Exited {
        /// Exit code.
        code: i32,
    },
    /// Process was terminated by a signal.
    Signaled {
        /// Signal number.
        signal: i32,
    },
    /// Status could not be determined.
    Unknown,
}

// ── ProcessRecord ─────────────────────────────────────────────────────────────

/// A single tracked process entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRecord {
    /// Process ID.
    pub pid: u32,
    /// Command that was executed.
    pub command: String,
    /// Arguments passed to the command.
    pub args: Vec<String>,
    /// When the process was spawned.
    pub started_at: DateTime<Utc>,
    /// Path to the agent's cgroup (Linux only). Used to read live CPU/memory stats.
    pub cgroup_path: Option<String>,
    /// Current lifecycle status.
    pub status: ProcessStatus,
    /// Last observed CPU usage in nanoseconds (from `cgroup cpu.stat`).
    pub cpu_usage_ns: Option<u64>,
    /// Last observed memory usage in bytes (from `cgroup memory.current`).
    pub memory_bytes: Option<u64>,
    /// Captured stdout/stderr output directory (non-interactive processes only).
    ///
    /// The `TempDir` inside is automatically deleted when the
    /// last `Arc` is dropped.
    #[serde(skip)]
    pub output: Option<Arc<CapturedOutput>>,
}

impl ProcessRecord {
    /// Create a new record for a just-spawned process.
    #[must_use]
    pub fn new(pid: u32, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            pid,
            command: command.into(),
            args,
            started_at: Utc::now(),
            cgroup_path: None,
            status: ProcessStatus::Running,
            cpu_usage_ns: None,
            memory_bytes: None,
            output: None,
        }
    }
}

// ── ProcessRegistry ───────────────────────────────────────────────────────────

/// Thread-safe in-memory store of tracked processes.
///
/// Intended to be wrapped in `Arc<tokio::sync::RwLock<ProcessRegistry>>` and
/// shared between the sandbox backend (writes) and LLM tools (reads).
#[derive(Debug, Default)]
pub struct ProcessRegistry {
    /// Entries keyed by PID.
    entries: HashMap<u32, ProcessRecord>,
    /// Optional cap on concurrent entries.
    max_tracked: Option<usize>,
}

impl ProcessRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new(max_tracked: Option<usize>) -> Self {
        Self {
            entries: HashMap::new(),
            max_tracked,
        }
    }

    /// Register a newly-spawned process.
    ///
    /// Returns [`SandboxError::RegistryFull`] if `max_tracked` would be exceeded.
    pub fn insert(&mut self, record: ProcessRecord) -> Result<(), SandboxError> {
        if let Some(max) = self.max_tracked {
            let running = self
                .entries
                .values()
                .filter(|r| r.status == ProcessStatus::Running)
                .count();
            if running >= max {
                return Err(SandboxError::RegistryFull { max_tracked: max });
            }
        }
        let _ = self.entries.insert(record.pid, record);
        Ok(())
    }

    /// Look up a process by PID.
    #[must_use]
    pub fn get(&self, pid: u32) -> Option<&ProcessRecord> {
        self.entries.get(&pid)
    }

    /// Mutably look up a process by PID.
    pub fn get_mut(&mut self, pid: u32) -> Option<&mut ProcessRecord> {
        self.entries.get_mut(&pid)
    }

    /// Return all currently-running processes.
    pub fn running(&self) -> impl Iterator<Item = &ProcessRecord> {
        self.entries
            .values()
            .filter(|r| r.status == ProcessStatus::Running)
    }

    /// Return all processes (running and exited).
    pub fn all(&self) -> impl Iterator<Item = &ProcessRecord> {
        self.entries.values()
    }

    /// Mark a process as exited.
    pub fn mark_exited(&mut self, pid: u32, code: i32) {
        if let Some(r) = self.entries.get_mut(&pid) {
            r.status = ProcessStatus::Exited { code };
        }
    }

    /// Mark a process as signaled.
    pub fn mark_signaled(&mut self, pid: u32, signal: i32) {
        if let Some(r) = self.entries.get_mut(&pid) {
            r.status = ProcessStatus::Signaled { signal };
        }
    }

    /// Remove all exited/signaled records. Useful for long-running agents to
    /// prevent unbounded growth.
    pub fn gc(&mut self) {
        self.entries
            .retain(|_, r| r.status == ProcessStatus::Running);
    }
}

// ── monitor_child ────────────────────────────────────────────────────────────

/// Spawn a background task that awaits `child` and updates `registry` when it
/// exits.
///
/// After calling this function the caller should no longer use `child` —
/// ownership transfers to the background task. Query
/// [`ProcessRegistry::get`] to observe the final status.
pub fn monitor_child(
    mut child: tokio::process::Child,
    pid: u32,
    registry: Arc<RwLock<ProcessRegistry>>,
) {
    let _handle = tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                let code = status
                    .code()
                    .unwrap_or_else(|| if status.success() { 0 } else { -1 });
                let mut reg = registry.write().await;
                if status.code().is_some() {
                    reg.mark_exited(pid, code);
                } else {
                    // Exited via signal (Unix), no code available.
                    reg.mark_signaled(pid, -1);
                }
            }
            Err(_) => {
                registry.write().await.mark_signaled(pid, -1);
            }
        }
    });
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_record(pid: u32) -> ProcessRecord {
        ProcessRecord::new(pid, "sleep", vec!["1".into()])
    }

    #[test]
    fn insert_and_retrieve() {
        let mut reg = ProcessRegistry::new(None);
        reg.insert(make_record(100)).unwrap();
        let r = reg.get(100).unwrap();
        assert_eq!(r.pid, 100);
        assert_eq!(r.status, ProcessStatus::Running);
    }

    #[test]
    fn max_tracked_enforced() {
        let mut reg = ProcessRegistry::new(Some(2));
        reg.insert(make_record(1)).unwrap();
        reg.insert(make_record(2)).unwrap();
        let err = reg.insert(make_record(3)).unwrap_err();
        assert!(matches!(err, SandboxError::RegistryFull { max_tracked: 2 }));
    }

    #[test]
    fn max_tracked_counts_only_running() {
        let mut reg = ProcessRegistry::new(Some(2));
        reg.insert(make_record(1)).unwrap();
        reg.insert(make_record(2)).unwrap();
        reg.mark_exited(1, 0);
        // Now only 1 running; inserting a third should succeed.
        reg.insert(make_record(3)).unwrap();
    }

    #[test]
    fn mark_exited_and_gc() {
        let mut reg = ProcessRegistry::new(None);
        reg.insert(make_record(10)).unwrap();
        reg.insert(make_record(11)).unwrap();
        reg.mark_exited(10, 0);
        reg.gc();
        assert!(reg.get(10).is_none());
        assert!(reg.get(11).is_some());
    }

    #[test]
    fn running_iterator_excludes_exited() {
        let mut reg = ProcessRegistry::new(None);
        reg.insert(make_record(20)).unwrap();
        reg.insert(make_record(21)).unwrap();
        reg.mark_signaled(20, 9);
        let running: Vec<_> = reg.running().collect();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].pid, 21);
    }
}

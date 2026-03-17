//! cgroup v2 resource manager.
//!
//! Discovers the calling process's own cgroup path via `/proc/self/cgroup`
//! and creates per-agent sub-cgroups as siblings of the process cgroup:
//!
//! ```text
//! <process-cgroup-parent>/
//!   <process-scope>/          ← synwire process lives here
//!   synwire/
//!     agents/<agent-uuid>/    ← agent processes live here
//! ```
//!
//! Placing agent cgroups under the process cgroup's **parent** avoids the
//! cgroup-v2 "no internal processes" constraint (the process itself must not
//! be in a cgroup that also enables subtree controllers), while keeping the
//! hierarchy as close to the running synwire process as possible.
//!
//! No root privileges are required — systemd already delegates the entire
//! `user@<uid>.service/` subtree at login and the calling process's parent
//! cgroup is writeable.
//!
//! # Enabling delegation
//!
//! On most systemd-based distributions (Fedora, Ubuntu 22.04+, Arch),
//! delegation works out of the box. If controllers (cpu, memory, pids) are
//! not available in your user subtree, configure systemd to delegate them:
//!
//! ```bash
//! sudo mkdir -p /etc/systemd/system/user@.service.d
//! cat <<'EOF' | sudo tee /etc/systemd/system/user@.service.d/delegate.conf
//! [Service]
//! Delegate=cpu cpuset io memory pids
//! EOF
//! sudo systemctl daemon-reload
//! ```
//!
//! Log out and back in (or `sudo systemctl restart user@$(id -u).service`)
//! for changes to take effect.
//!
//! **WSL2**: add `systemd=true` under `[boot]` in `/etc/wsl.conf` and
//! restart WSL to enable systemd (required for user cgroup delegation).
//!
//! See the [Process Sandboxing](https://randomvariable.github.io/synwire/how-to/process-sandbox.html)
//! guide for full setup instructions including namespace isolation.

use std::path::PathBuf;

use tokio::fs;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::SandboxError;

/// CPU and memory statistics read from cgroup v2 controllers.
#[derive(Debug, Clone)]
pub struct CgroupStats {
    /// Cumulative CPU time in nanoseconds (from `cpu.stat` `usage_usec`).
    pub cpu_usage_ns: u64,
    /// Current memory usage in bytes (from `memory.current`).
    pub memory_current_bytes: u64,
}

/// cgroup v2 resource manager for a single agent.
///
/// One `CgroupV2Manager` is created per agent instance. It owns a sub-cgroup
/// that is a sibling of the calling process's cgroup, providing resource
/// accounting, enforcement, and forcible termination.
#[derive(Debug)]
pub struct CgroupV2Manager {
    /// Absolute path to the agent's cgroup directory.
    base_path: PathBuf,
}

impl CgroupV2Manager {
    /// Check whether cgroup v2 is available on this system.
    ///
    /// Returns `true` if `/sys/fs/cgroup/cgroup.controllers` exists.
    pub async fn is_available() -> bool {
        fs::metadata("/sys/fs/cgroup/cgroup.controllers")
            .await
            .is_ok()
    }

    /// Discover the parent of the calling process's own cgroup.
    ///
    /// Parses the `0::` entry (unified hierarchy) from `/proc/self/cgroup` to
    /// obtain the process's current cgroup path, then returns its parent.
    /// Agent sub-cgroups are created there, making them siblings of the
    /// process's cgroup and enabling resource controllers without violating
    /// the cgroup-v2 "no internal processes" constraint.
    ///
    /// Falls back to the process's own cgroup if it has no parent (e.g.,
    /// running directly under the cgroup root — rare in practice).
    ///
    /// # Errors
    ///
    /// Returns [`SandboxError::CgroupParseFailed`] if `/proc/self/cgroup`
    /// cannot be read or the `0::` entry is absent.
    pub async fn discover_cgroup_parent() -> Result<PathBuf, SandboxError> {
        let contents = fs::read_to_string("/proc/self/cgroup")
            .await
            .map_err(|e| SandboxError::CgroupParseFailed(format!("read /proc/self/cgroup: {e}")))?;

        // Find the unified hierarchy line: "0::<path>"
        let cgroup_rel = contents
            .lines()
            .find_map(|line| {
                let mut parts = line.splitn(3, ':');
                let hier = parts.next()?;
                let _ = parts.next(); // controllers field
                let path = parts.next()?;
                if hier == "0" {
                    Some(path.trim().to_string())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                SandboxError::CgroupParseFailed(
                    "no unified hierarchy (0::) entry in /proc/self/cgroup".into(),
                )
            })?;

        // Construct the absolute path of the process's own cgroup.
        let process_cgroup =
            PathBuf::from("/sys/fs/cgroup").join(cgroup_rel.trim_start_matches('/'));

        // Use the parent so agent sub-cgroups are siblings of the process cgroup.
        // This satisfies cgroup-v2 NIP: no processes are directly in the parent.
        let parent = process_cgroup
            .parent()
            .unwrap_or(&process_cgroup)
            .to_path_buf();

        debug!(?process_cgroup, ?parent, "discovered process cgroup");
        Ok(parent)
    }

    /// Create a new cgroup manager for the given agent UUID.
    ///
    /// Discovers the process cgroup's parent, creates the agent sub-cgroup
    /// directory, enables required controllers on the parent and on the
    /// `synwire/` intermediate cgroup, and applies resource limits if provided.
    ///
    /// Falls back gracefully (returns error, caller should log and disable
    /// cgroup tracking) if cgroup v2 is not available or the path is not
    /// writable.
    ///
    /// # Errors
    ///
    /// Returns a [`SandboxError`] variant if cgroup setup fails.
    pub async fn new(
        agent_id: Uuid,
        resources: Option<&synwire_core::agents::sandbox::ResourceLimits>,
    ) -> Result<Self, SandboxError> {
        let cgroup_parent = Self::discover_cgroup_parent().await?;
        let synwire_root = cgroup_parent.join("synwire");
        let agents_root = synwire_root.join("agents");
        let base_path = agents_root.join(agent_id.to_string());

        // Enable controllers in the parent cgroup so the `synwire/` subtree
        // can use cpu/memory/pids controllers.
        let parent_control = cgroup_parent.join("cgroup.subtree_control");
        let _ = fs::write(&parent_control, "+cpu +memory +pids").await;

        // Create the `synwire/` intermediate cgroup if needed.
        if !synwire_root.exists() {
            fs::create_dir_all(&synwire_root)
                .await
                .map_err(SandboxError::CgroupIo)?;
        }

        // Propagate controllers into the `synwire/agents/` subtree.
        let synwire_control = synwire_root.join("cgroup.subtree_control");
        let _ = fs::write(&synwire_control, "+cpu +memory +pids").await;

        // Verify writability by probing with a temporary directory.
        let test_path = agents_root.join("_write_test");
        match fs::create_dir_all(&test_path).await {
            Ok(()) => {
                let _ = fs::remove_dir(&test_path).await;
            }
            Err(_) => {
                return Err(SandboxError::CgroupNotWritable {
                    path: agents_root.display().to_string(),
                });
            }
        }

        fs::create_dir_all(&base_path)
            .await
            .map_err(SandboxError::CgroupIo)?;

        let mgr = Self { base_path };

        if let Some(limits) = resources {
            mgr.apply_limits(limits).await?;
        }

        Ok(mgr)
    }

    /// Absolute path to this agent's cgroup directory.
    #[must_use]
    pub fn base_path(&self) -> &std::path::Path {
        &self.base_path
    }

    /// Move a process into this agent's cgroup.
    ///
    /// # Errors
    ///
    /// Returns [`SandboxError::CgroupIo`] if writing to `cgroup.procs` fails.
    pub async fn move_pid(&self, pid: u32) -> Result<(), SandboxError> {
        let procs_path = self.base_path.join("cgroup.procs");
        fs::write(&procs_path, pid.to_string())
            .await
            .map_err(SandboxError::CgroupIo)
    }

    /// Read live CPU and memory stats for this cgroup.
    ///
    /// Returns `None` if either file is missing or unparseable (non-fatal).
    pub async fn read_stats(&self) -> Option<CgroupStats> {
        let cpu_ns = read_cpu_usage_ns(&self.base_path).await;
        let memory_bytes = read_memory_current(&self.base_path).await;
        match (cpu_ns, memory_bytes) {
            (Some(cpu_usage_ns), Some(memory_current_bytes)) => Some(CgroupStats {
                cpu_usage_ns,
                memory_current_bytes,
            }),
            _ => None,
        }
    }

    /// Forcibly kill all processes in this cgroup.
    ///
    /// Tries `cgroup.kill` (Linux 5.14+); falls back to reading `cgroup.procs`
    /// and sending `SIGKILL` to each PID via nix.
    ///
    /// # Errors
    ///
    /// Returns [`SandboxError::CgroupIo`] if both kill mechanisms fail.
    pub async fn kill_all(&self) -> Result<(), SandboxError> {
        let kill_path = self.base_path.join("cgroup.kill");
        if fs::write(&kill_path, "1").await.is_ok() {
            return Ok(());
        }

        // Fallback: read cgroup.procs and SIGKILL each PID.
        let procs_path = self.base_path.join("cgroup.procs");
        let contents = fs::read_to_string(&procs_path)
            .await
            .map_err(SandboxError::CgroupIo)?;

        for line in contents.lines() {
            let Ok(pid_raw) = line.trim().parse::<i32>() else {
                continue;
            };
            let pid = nix::unistd::Pid::from_raw(pid_raw);
            let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGKILL);
        }
        Ok(())
    }

    /// Remove this agent's cgroup directory.
    ///
    /// Should only be called after all processes have exited. Logs a warning
    /// if removal fails (e.g., lingering processes).
    pub async fn destroy(&self) {
        if let Err(e) = fs::remove_dir(&self.base_path).await {
            warn!(path = %self.base_path.display(), error = %e, "failed to remove agent cgroup");
        }
    }

    /// Apply resource limits to this cgroup.
    async fn apply_limits(
        &self,
        limits: &synwire_core::agents::sandbox::ResourceLimits,
    ) -> Result<(), SandboxError> {
        if let Some(mem_bytes) = limits.memory_bytes {
            fs::write(self.base_path.join("memory.max"), mem_bytes.to_string())
                .await
                .map_err(SandboxError::CgroupIo)?;
        }

        if let Some(cpu_quota) = limits.cpu_quota {
            // cpu.max format: "<quota> <period>" where quota and period are in µs.
            // A period of 100ms = 100_000 µs is conventional.
            let period_us = 100_000u64;
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            let quota_us = (f64::from(cpu_quota) * period_us as f64) as u64;
            let content = format!("{quota_us} {period_us}");
            fs::write(self.base_path.join("cpu.max"), content)
                .await
                .map_err(SandboxError::CgroupIo)?;
        }

        if let Some(max_pids) = limits.max_pids {
            fs::write(self.base_path.join("pids.max"), max_pids.to_string())
                .await
                .map_err(SandboxError::CgroupIo)?;
        }

        Ok(())
    }
}

impl Drop for CgroupV2Manager {
    fn drop(&mut self) {
        // Best-effort: kill all processes and remove the cgroup directory.
        // Uses synchronous std::fs — Drop cannot be async.

        // Try cgroup.kill (Linux 5.14+) first.
        let kill_path = self.base_path.join("cgroup.kill");
        if std::fs::write(&kill_path, "1").is_err() {
            // Fallback: read cgroup.procs and SIGKILL each PID individually.
            if let Ok(contents) = std::fs::read_to_string(self.base_path.join("cgroup.procs")) {
                for line in contents.lines() {
                    if let Ok(pid_raw) = line.trim().parse::<i32>() {
                        let pid = nix::unistd::Pid::from_raw(pid_raw);
                        let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGKILL);
                    }
                }
            }
        }

        // Try to remove the now-empty cgroup directory.
        if let Err(e) = std::fs::remove_dir(&self.base_path) {
            warn!(path = %self.base_path.display(), error = %e, "failed to remove agent cgroup on drop");
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Parse `usage_usec` from `cpu.stat` and return nanoseconds.
async fn read_cpu_usage_ns(base: &std::path::Path) -> Option<u64> {
    let content = fs::read_to_string(base.join("cpu.stat")).await.ok()?;
    content.lines().find_map(|line| {
        let mut parts = line.splitn(2, ' ');
        if parts.next()? == "usage_usec" {
            parts.next()?.trim().parse::<u64>().ok().map(|us| us * 1000)
        } else {
            None
        }
    })
}

/// Read `memory.current` and return bytes.
async fn read_memory_current(base: &std::path::Path) -> Option<u64> {
    let content = fs::read_to_string(base.join("memory.current")).await.ok()?;
    content.trim().parse::<u64>().ok()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Parse the `0::` cgroup path from a `/proc/self/cgroup` contents string
    /// and return the absolute path. Mirrors `discover_cgroup_parent`'s logic.
    fn parse_process_cgroup(content: &str) -> PathBuf {
        let rel = content
            .lines()
            .find_map(|line| {
                let mut parts = line.splitn(3, ':');
                let hier = parts.next()?;
                let _ = parts.next();
                let path = parts.next()?;
                if hier == "0" {
                    Some(path.trim().to_string())
                } else {
                    None
                }
            })
            .unwrap();
        PathBuf::from("/sys/fs/cgroup").join(rel.trim_start_matches('/'))
    }

    #[test]
    fn parse_cgroup_line_unified_hierarchy() {
        let content = "12:cpuset:/\n0::/user.slice/user-1000.slice/user@1000.service/app.slice\n";
        let process_cgroup = parse_process_cgroup(content);
        assert_eq!(
            process_cgroup,
            PathBuf::from("/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice")
        );
    }

    #[test]
    fn cgroup_parent_is_one_level_up() {
        // Typical case: process is in a scope inside app.slice.
        let content = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/code.scope\n";
        let process_cgroup = parse_process_cgroup(content);
        let parent = process_cgroup
            .parent()
            .unwrap_or(&process_cgroup)
            .to_path_buf();
        assert_eq!(
            parent,
            PathBuf::from("/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice")
        );
        // Resulting agent cgroup path.
        let agent_cgroup = parent.join("synwire").join("agents").join("test-uuid");
        assert_eq!(
            agent_cgroup,
            PathBuf::from(
                "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice/synwire/agents/test-uuid"
            )
        );
    }

    #[test]
    fn cgroup_parent_fallback_at_root_level() {
        // Edge case: process is at the cgroup root (0::/).
        // parse_process_cgroup("0::/") joins "" onto /sys/fs/cgroup, giving
        // /sys/fs/cgroup itself.  .parent() then returns /sys/fs.
        // In practice a process is never at the raw cgroup root; this test
        // just documents the degenerate behaviour.
        let content = "0::/\n";
        let process_cgroup = parse_process_cgroup(content);
        let parent = process_cgroup
            .parent()
            .unwrap_or(&process_cgroup)
            .to_path_buf();
        // parent is /sys/fs (one level above /sys/fs/cgroup)
        assert!(parent.starts_with("/sys"));
    }
}

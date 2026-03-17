//! Process management backend.

use std::collections::HashMap;

use tokio::sync::Mutex;

use serde::{Deserialize, Serialize};
use synwire_core::BoxFuture;
use synwire_core::vfs::error::VfsError;
use synwire_core::vfs::types::{ExecuteResponse, JobInfo, ProcessInfo};
use tokio::process::Command;
use uuid::Uuid;

/// Process management backend.
#[derive(Debug)]
pub struct ProcessManager {
    jobs: Mutex<HashMap<String, JobInfo>>,
    /// Background child handles (`job_id` → `pid`).
    bg_pids: Mutex<HashMap<String, u32>>,
}

/// Process listing entry from the OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessList {
    /// Running processes.
    pub processes: Vec<ProcessInfo>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    /// Create a new process backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
            bg_pids: Mutex::new(HashMap::new()),
        }
    }

    /// List running processes using `ps`.
    pub fn list_processes(&self) -> BoxFuture<'_, Result<ProcessList, VfsError>> {
        Box::pin(async move {
            let output = Command::new("ps")
                .args(["-eo", "pid,ppid,comm,pcpu,rss,stat"])
                .output()
                .await
                .map_err(VfsError::Io)?;

            let text = String::from_utf8_lossy(&output.stdout);
            let mut processes = Vec::new();
            for line in text.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() < 6 {
                    continue;
                }
                let pid = cols[0].parse::<u32>().unwrap_or(0);
                let parent_pid = cols[1].parse::<u32>().ok();
                let command = cols[2].to_string();
                let cpu_pct = cols[3].parse::<f32>().ok();
                let mem_kb = cols[4].parse::<u64>().ok();
                let state = cols[5].to_string();
                processes.push(ProcessInfo {
                    pid,
                    command,
                    cpu_pct,
                    mem_bytes: mem_kb.map(|k| k * 1024),
                    parent_pid,
                    state,
                });
            }
            Ok(ProcessList { processes })
        })
    }

    /// Kill a process by PID.
    pub fn kill_process(&self, pid: u32) -> BoxFuture<'_, Result<(), VfsError>> {
        Box::pin(async move {
            let output = Command::new("kill")
                .arg(pid.to_string())
                .output()
                .await
                .map_err(VfsError::Io)?;
            if !output.status.success() {
                return Err(VfsError::NotFound(format!("process {pid}")));
            }
            Ok(())
        })
    }

    /// Spawn a background job and return its job ID.
    pub fn spawn_background<'a>(
        &'a self,
        cmd: &'a str,
        args: &'a [String],
    ) -> BoxFuture<'a, Result<String, VfsError>> {
        Box::pin(async move {
            let job_id = Uuid::new_v4().to_string();
            let child = Command::new(cmd).args(args).spawn().map_err(VfsError::Io)?;

            let pid = child.id();
            let _ = self.jobs.lock().await.insert(
                job_id.clone(),
                JobInfo {
                    id: job_id.clone(),
                    pid,
                    command: format!("{cmd} {}", args.join(" ")),
                    status: "running".to_string(),
                },
            );

            if let Some(pid) = pid {
                let _ = self.bg_pids.lock().await.insert(job_id.clone(), pid);
            }
            Ok(job_id)
        })
    }

    /// List background jobs.
    pub async fn list_jobs(&self) -> Vec<JobInfo> {
        self.jobs.lock().await.values().cloned().collect()
    }

    /// Execute a command and wait for it.
    pub fn execute<'a>(
        &'a self,
        cmd: &'a str,
        args: &'a [String],
    ) -> BoxFuture<'a, Result<ExecuteResponse, VfsError>> {
        Box::pin(async move {
            let output = Command::new(cmd)
                .args(args)
                .output()
                .await
                .map_err(VfsError::Io)?;
            Ok(ExecuteResponse {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_processes_returns_data() {
        let backend = ProcessManager::new();
        let list = backend.list_processes().await.expect("list_processes");
        // On any Unix system this should return at least 1 process.
        assert!(!list.processes.is_empty());
    }

    #[tokio::test]
    async fn test_spawn_background_job() {
        let backend = ProcessManager::new();
        let job_id = backend
            .spawn_background("sleep", &["60".to_string()])
            .await
            .expect("spawn");
        let jobs = backend.list_jobs().await;
        assert!(jobs.iter().any(|j| j.id == job_id));
        // Clean up: kill the background process so no leaked subprocess remains.
        let pid = backend.bg_pids.lock().await.get(&job_id).copied();
        if let Some(pid) = pid {
            let _ = backend.kill_process(pid).await;
        }
    }

    #[tokio::test]
    async fn test_execute_returns_output() {
        let backend = ProcessManager::new();
        let resp = backend
            .execute("echo", &["hello".to_string()])
            .await
            .expect("execute");
        assert_eq!(resp.exit_code, 0);
        assert!(resp.stdout.contains("hello"));
    }
}

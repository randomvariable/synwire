//! External process runtime.
//!
//! Executes skills by spawning a subprocess, capturing stdout, stderr, and the
//! exit code. The input `args` JSON must contain a `command` string field, and
//! may optionally include `args` (array of strings), `env` (string-to-string
//! map), `cwd` (string), and `stdin` (string).
//!
//! When the `sandboxed` feature is enabled and a
//! [`ProcessRegistry`](synwire_sandbox::ProcessRegistry) is provided, spawned
//! processes are tracked in the registry and monitored for exit.

use std::collections::HashMap;
use std::io::Write as _;
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;
use tracing::debug;

use crate::error::SkillError;
use crate::runtime::{SkillExecutor, SkillInput, SkillOutput};

/// Parameters extracted from the input JSON for external process execution.
#[derive(Debug, Deserialize)]
struct ExternalArgs {
    /// The command to execute.
    command: String,
    /// Arguments to pass to the command.
    #[serde(default)]
    args: Vec<String>,
    /// Environment variables to set.
    #[serde(default)]
    env: HashMap<String, String>,
    /// Working directory for the subprocess.
    #[serde(default)]
    cwd: Option<String>,
    /// Data to write to the subprocess stdin.
    #[serde(default)]
    stdin: Option<String>,
}

/// Executor that spawns an external process.
///
/// When the `sandboxed` feature is enabled and a registry is provided via
/// [`ExternalRuntime::with_registry`], spawned processes are tracked in the
/// [`ProcessRegistry`](synwire_sandbox::ProcessRegistry). Otherwise, processes
/// are spawned directly with [`std::process::Command`].
///
/// # Example input
///
/// ```json
/// {
///     "command": "echo",
///     "args": ["hello"],
///     "env": { "MY_VAR": "value" },
///     "cwd": "/tmp",
///     "stdin": null
/// }
/// ```
#[derive(Debug)]
pub struct ExternalRuntime {
    /// Maximum time the subprocess is allowed to run before being killed.
    timeout: Duration,
    /// Optional sandbox process registry for tracking spawned processes.
    #[cfg(feature = "sandboxed")]
    registry: Option<std::sync::Arc<tokio::sync::RwLock<synwire_sandbox::ProcessRegistry>>>,
}

impl Default for ExternalRuntime {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            #[cfg(feature = "sandboxed")]
            registry: None,
        }
    }
}

impl ExternalRuntime {
    /// Create a new [`ExternalRuntime`] with the default 30-second timeout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new [`ExternalRuntime`] with a custom timeout.
    pub const fn with_timeout_secs(timeout_secs: u64) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_secs),
            #[cfg(feature = "sandboxed")]
            registry: None,
        }
    }

    /// Create a new [`ExternalRuntime`] with a sandbox process registry.
    ///
    /// Spawned processes will be registered and tracked. The registry is
    /// shared with the caller for observation (e.g. `list_processes`,
    /// `kill_process`).
    #[cfg(feature = "sandboxed")]
    pub const fn with_registry(
        registry: std::sync::Arc<tokio::sync::RwLock<synwire_sandbox::ProcessRegistry>>,
    ) -> Self {
        Self {
            timeout: Duration::from_secs(30),
            registry: Some(registry),
        }
    }

    /// Create a new [`ExternalRuntime`] with both a custom timeout and a
    /// sandbox process registry.
    #[cfg(feature = "sandboxed")]
    pub const fn with_timeout_and_registry(
        timeout_secs: u64,
        registry: std::sync::Arc<tokio::sync::RwLock<synwire_sandbox::ProcessRegistry>>,
    ) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_secs),
            registry: Some(registry),
        }
    }

    /// Spawn the process and wait for completion, returning (stdout, stderr, `exit_code`).
    fn spawn_and_wait(
        &self,
        ext_args: &ExternalArgs,
    ) -> Result<(String, String, Option<i32>), SkillError> {
        let mut cmd = Command::new(&ext_args.command);
        let _ = cmd.args(&ext_args.args);
        let _ = cmd.envs(&ext_args.env);

        if let Some(ref cwd) = ext_args.cwd {
            let _ = cmd.current_dir(cwd);
        }

        // If stdin data is provided, pipe it in.
        if ext_args.stdin.is_some() {
            let _ = cmd.stdin(std::process::Stdio::piped());
        }
        let _ = cmd.stdout(std::process::Stdio::piped());
        let _ = cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;

        // Register with sandbox if available.
        #[cfg(feature = "sandboxed")]
        self.register_process(&child, ext_args);

        // Write stdin data if provided.
        if let Some(ref stdin_data) = ext_args.stdin {
            if let Some(ref mut stdin_handle) = child.stdin {
                // Write and drop to close the pipe; ignore errors since the
                // process may have already exited.
                let _ = stdin_handle.write_all(stdin_data.as_bytes());
            }
            // Drop stdin to signal EOF.
            drop(child.stdin.take());
        }

        // Wait with timeout by polling in a loop. `std::process::Command` does
        // not natively support timeouts, so we poll with a short sleep.
        let deadline = std::time::Instant::now() + self.timeout;
        #[allow(unused_variables)]
        let pid = child.id();
        let output = loop {
            if let Some(status) = child.try_wait()? {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(ref mut out) = child.stdout {
                    let _ = std::io::Read::read_to_string(out, &mut stdout)?;
                }
                if let Some(ref mut err) = child.stderr {
                    let _ = std::io::Read::read_to_string(err, &mut stderr)?;
                }

                // Update registry with exit status.
                #[cfg(feature = "sandboxed")]
                self.mark_exited(pid, status.code().unwrap_or(-1));

                break (stdout, stderr, status.code());
            } else if std::time::Instant::now() >= deadline {
                // Attempt to kill the process.
                let _ = child.kill();
                let _ = child.wait();

                // Update registry for killed process.
                #[cfg(feature = "sandboxed")]
                self.mark_exited(pid, -1);

                return Err(SkillError::Runtime {
                    runtime: "external".to_owned(),
                    message: format!(
                        "process '{}' timed out after {} seconds",
                        ext_args.command,
                        self.timeout.as_secs()
                    ),
                });
            }
            std::thread::sleep(Duration::from_millis(50));
        };

        Ok(output)
    }

    /// Register a child process in the sandbox registry (sandboxed feature only).
    #[cfg(feature = "sandboxed")]
    fn register_process(&self, child: &std::process::Child, ext_args: &ExternalArgs) {
        if let Some(ref registry) = self.registry {
            let record = synwire_sandbox::ProcessRecord::new(
                child.id(),
                &ext_args.command,
                ext_args.args.clone(),
            );
            // Best-effort registration: if the registry is full, log and continue.
            if let Ok(mut reg) = registry.try_write() {
                if let Err(e) = reg.insert(record) {
                    tracing::warn!(
                        error = %e,
                        pid = child.id(),
                        "failed to register process in sandbox registry"
                    );
                }
            } else {
                tracing::debug!(
                    pid = child.id(),
                    "could not acquire registry write lock; skipping registration"
                );
            }
        }
    }

    /// Mark a process as exited in the sandbox registry (sandboxed feature only).
    #[cfg(feature = "sandboxed")]
    fn mark_exited(&self, pid: u32, code: i32) {
        if let Some(ref registry) = self.registry
            && let Ok(mut reg) = registry.try_write()
        {
            reg.mark_exited(pid, code);
        }
    }
}

impl SkillExecutor for ExternalRuntime {
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError> {
        let ext_args: ExternalArgs = serde_json::from_value(input.args).map_err(|e| {
            SkillError::InvalidManifest(format!("invalid external runtime args: {e}"))
        })?;

        debug!(
            command = %ext_args.command,
            args = ?ext_args.args,
            timeout_secs = self.timeout.as_secs(),
            "spawning external process"
        );

        let (stdout, stderr, exit_code) = self.spawn_and_wait(&ext_args)?;

        Ok(SkillOutput {
            result: serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": exit_code,
            }),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::runtime::SkillInput;

    #[test]
    fn echo_captures_stdout() {
        let runtime = ExternalRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({
                "command": "echo",
                "args": ["hello"]
            }),
        };
        let output = runtime.execute(input).expect("echo should succeed");
        let stdout = output.result["stdout"]
            .as_str()
            .expect("stdout should be a string");
        assert_eq!(stdout.trim(), "hello");
        assert_eq!(output.result["exit_code"], 0);
    }

    #[test]
    fn missing_command_field_returns_error() {
        let runtime = ExternalRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({}),
        };
        let err = runtime
            .execute(input)
            .expect_err("missing command should fail");
        assert!(
            matches!(err, SkillError::InvalidManifest(_)),
            "expected InvalidManifest, got {err}"
        );
    }

    #[test]
    fn stdin_is_forwarded() {
        let runtime = ExternalRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({
                "command": "cat",
                "stdin": "piped data"
            }),
        };
        let output = runtime.execute(input).expect("cat should succeed");
        let stdout = output.result["stdout"]
            .as_str()
            .expect("stdout should be a string");
        assert_eq!(stdout, "piped data");
    }

    #[test]
    fn nonexistent_command_returns_io_error() {
        let runtime = ExternalRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({
                "command": "/nonexistent/binary/xyz"
            }),
        };
        let err = runtime
            .execute(input)
            .expect_err("nonexistent binary should fail");
        assert!(
            matches!(err, SkillError::Io(_)),
            "expected Io error, got {err}"
        );
    }

    #[cfg(feature = "sandboxed")]
    #[test]
    fn sandboxed_echo_registers_process() {
        let registry = std::sync::Arc::new(tokio::sync::RwLock::new(
            synwire_sandbox::ProcessRegistry::new(None),
        ));
        let runtime = ExternalRuntime::with_registry(std::sync::Arc::clone(&registry));
        let input = SkillInput {
            args: serde_json::json!({
                "command": "echo",
                "args": ["tracked"]
            }),
        };
        let output = runtime
            .execute(input)
            .expect("sandboxed echo should succeed");
        let stdout = output.result["stdout"]
            .as_str()
            .expect("stdout should be a string");
        assert_eq!(stdout.trim(), "tracked");

        // The process should have been registered and marked exited.
        let reg = registry.try_read().expect("should acquire read lock");
        // At least one process was tracked (it may have been garbage-collected
        // but the entry should still be present since we don't gc here).
        assert!(reg.all().count() >= 1);
        drop(reg);
    }
}

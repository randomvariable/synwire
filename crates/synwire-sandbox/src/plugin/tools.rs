#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::match_same_arms
)]
//! LLM-callable tools for process management.
//!
//! These tools are contributed to the agent's tool registry by
//! [`ProcessPlugin`](super::ProcessPlugin) when `process_tracking.enabled = true`.
//!
//! - [`ListProcessesTool`] — list running processes (own + visible children)
//! - [`KillProcessTool`] — send a signal to a process (own only)
//! - [`ProcessStatsTool`] — detailed stats for one process (own + children)
//! - [`WaitForProcessTool`] — poll until a process exits (own + children)
//! - [`ReadProcessOutputTool`] — read captured stdout/stderr (own + children)

use std::sync::OnceLock;

use serde_json::{Value, json};

use synwire_core::BoxFuture;
use synwire_core::error::{SynwireError, ToolError};
use synwire_core::tools::{Tool, ToolOutput, ToolResultStatus, ToolSchema};

use crate::process_registry::ProcessStatus;
use crate::visibility::ProcessVisibilityScope;

// ── helpers ────────────────────────────────────────────────────────────────────

fn tool_err(msg: impl Into<String>) -> SynwireError {
    SynwireError::Tool(ToolError::InvocationFailed {
        message: msg.into(),
    })
}

fn validation_err(msg: impl Into<String>) -> SynwireError {
    SynwireError::Tool(ToolError::ValidationFailed {
        message: msg.into(),
    })
}

fn parse_pid(input: &Value) -> Result<u32, SynwireError> {
    input["pid"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| validation_err("'pid' must be a positive integer"))
}

// ── ListProcessesTool ──────────────────────────────────────────────────────────

/// LLM tool: list all running processes visible to this agent.
///
/// For parent agents, includes processes from child agents (tagged with agent
/// label). For child agents, only shows own processes.
pub struct ListProcessesTool {
    scope: ProcessVisibilityScope,
    schema: OnceLock<ToolSchema>,
}

impl ListProcessesTool {
    /// Create a new `list_processes` tool backed by the given visibility scope.
    pub const fn new(scope: ProcessVisibilityScope) -> Self {
        Self {
            scope,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ListProcessesTool {
    fn name(&self) -> &'static str {
        "list_processes"
    }

    fn description(&self) -> &'static str {
        "List all running processes spawned by this agent and any visible sub-agents. \
         Returns PID, command, agent label, and live CPU/memory statistics when available."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "list_processes".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        })
    }

    fn invoke(&self, _input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let visible = self.scope.visible_running().await;
            let processes: Vec<serde_json::Value> = visible
                .iter()
                .map(|(label, r)| {
                    json!({
                        "pid": r.pid,
                        "command": format!("{} {}", r.command, r.args.join(" ")),
                        "agent": label.as_deref().unwrap_or("self"),
                        "cpu_pct": r.cpu_usage_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                        "mem_bytes": r.memory_bytes,
                        "state": "running",
                    })
                })
                .collect();

            let content =
                serde_json::to_string_pretty(&processes).map_err(|e| tool_err(e.to_string()))?;
            Ok(ToolOutput {
                content,
                ..Default::default()
            })
        })
    }
}

// ── KillProcessTool ────────────────────────────────────────────────────────────

/// LLM tool: send a signal to a process owned by this agent.
///
/// Only processes in this agent's own registry can be targeted — child-agent
/// processes cannot be killed from the parent.
pub struct KillProcessTool {
    scope: ProcessVisibilityScope,
    schema: OnceLock<ToolSchema>,
}

impl KillProcessTool {
    /// Create a new `kill_process` tool.
    pub const fn new(scope: ProcessVisibilityScope) -> Self {
        Self {
            scope,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for KillProcessTool {
    fn name(&self) -> &'static str {
        "kill_process"
    }

    fn description(&self) -> &'static str {
        "Send a signal to a process spawned by this agent. Defaults to SIGTERM. \
         Use SIGKILL only if the process does not respond to SIGTERM. \
         Only processes tracked by this agent can be targeted (not sub-agent processes)."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "kill_process".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pid": {
                        "type": "integer",
                        "description": "The process ID to signal.",
                        "minimum": 1
                    },
                    "signal": {
                        "type": "string",
                        "description": "Signal name (SIGTERM, SIGKILL, SIGINT, SIGHUP). Defaults to SIGTERM.",
                        "enum": ["SIGTERM", "SIGKILL", "SIGINT", "SIGHUP", "SIGSTOP", "SIGCONT"],
                        "default": "SIGTERM"
                    }
                },
                "required": ["pid"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let pid = parse_pid(&input)?;
            let signal_name = input["signal"].as_str().unwrap_or("SIGTERM").to_uppercase();

            // Kill only targets own registry.
            {
                let reg = self.scope.own.read().await;
                if reg.get(pid).is_none() {
                    return Ok(ToolOutput {
                        content: format!("process {pid} is not tracked by this agent"),
                        status: ToolResultStatus::Failure,
                        ..Default::default()
                    });
                }
                if reg.get(pid).map(|r| &r.status) != Some(&ProcessStatus::Running) {
                    return Ok(ToolOutput {
                        content: format!("process {pid} is not running"),
                        status: ToolResultStatus::Failure,
                        ..Default::default()
                    });
                }
            }

            send_signal_to_pid(pid, &signal_name)?;

            let signal_num = signal_name_to_number(&signal_name);
            {
                let mut reg = self.scope.own.write().await;
                reg.mark_signaled(pid, signal_num);
            }

            Ok(ToolOutput {
                content: format!("sent {signal_name} to process {pid}"),
                ..Default::default()
            })
        })
    }
}

// ── ProcessStatsTool ──────────────────────────────────────────────────────────

/// LLM tool: detailed stats for a single tracked process (own or child).
pub struct ProcessStatsTool {
    scope: ProcessVisibilityScope,
    schema: OnceLock<ToolSchema>,
}

impl ProcessStatsTool {
    /// Create a new `process_stats` tool.
    pub const fn new(scope: ProcessVisibilityScope) -> Self {
        Self {
            scope,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ProcessStatsTool {
    fn name(&self) -> &'static str {
        "process_stats"
    }

    fn description(&self) -> &'static str {
        "Retrieve detailed statistics for a specific process tracked by this agent \
         or a visible sub-agent, including PID, command, start time, CPU usage, \
         memory usage, and current status."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "process_stats".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pid": {
                        "type": "integer",
                        "description": "The process ID to query.",
                        "minimum": 1
                    }
                },
                "required": ["pid"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let pid = parse_pid(&input)?;

            let (agent_label, record) =
                self.scope.find(pid).await.ok_or_else(|| {
                    tool_err(format!("process {pid} is not visible to this agent"))
                })?;

            let status_str = match &record.status {
                ProcessStatus::Running => "running".to_string(),
                ProcessStatus::Exited { code } => format!("exited({code})"),
                ProcessStatus::Signaled { signal } => format!("signaled({signal})"),
                ProcessStatus::Unknown => "unknown".to_string(),
            };

            let stats = json!({
                "pid": record.pid,
                "command": record.command,
                "args": record.args,
                "agent": agent_label.as_deref().unwrap_or("self"),
                "started_at": record.started_at.to_rfc3339(),
                "status": status_str,
                "cpu_usage_ns": record.cpu_usage_ns,
                "memory_bytes": record.memory_bytes,
                "cgroup_path": record.cgroup_path,
                "has_captured_output": record.output.is_some(),
            });

            let content =
                serde_json::to_string_pretty(&stats).map_err(|e| tool_err(e.to_string()))?;
            Ok(ToolOutput {
                content,
                ..Default::default()
            })
        })
    }
}

// ── WaitForProcessTool ────────────────────────────────────────────────────────

/// LLM tool: wait for a process to finish and return its exit status.
///
/// Polls the registry every 200 ms until the process is no longer running or
/// the timeout expires. Requires that
/// [`monitor_child`](crate::process_registry::monitor_child) was called so the
/// registry is updated automatically when the process exits.
pub struct WaitForProcessTool {
    scope: ProcessVisibilityScope,
    schema: OnceLock<ToolSchema>,
}

impl WaitForProcessTool {
    /// Create a new `wait_for_process` tool.
    pub const fn new(scope: ProcessVisibilityScope) -> Self {
        Self {
            scope,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for WaitForProcessTool {
    fn name(&self) -> &'static str {
        "wait_for_process"
    }

    fn description(&self) -> &'static str {
        "Wait for a process to finish and return its exit status. \
         Defaults to 30 second timeout. Returns the final status \
         (exit code or signal) when the process completes, or \
         'timeout' if it is still running after the deadline."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "wait_for_process".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pid": {
                        "type": "integer",
                        "description": "The process ID to wait for.",
                        "minimum": 1
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Maximum time to wait in milliseconds (default: 30000).",
                        "minimum": 100,
                        "default": 30000
                    }
                },
                "required": ["pid"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let pid = parse_pid(&input)?;
            let timeout_ms = input["timeout_ms"].as_u64().unwrap_or(30_000);

            let start = std::time::Instant::now();
            loop {
                if let Some((_label, record)) = self.scope.find(pid).await {
                    match &record.status {
                        ProcessStatus::Running => { /* keep polling */ }
                        ProcessStatus::Exited { code } => {
                            return Ok(ToolOutput {
                                content: format!("exited with code {code}"),
                                ..Default::default()
                            });
                        }
                        ProcessStatus::Signaled { signal } => {
                            return Ok(ToolOutput {
                                content: format!("killed by signal {signal}"),
                                ..Default::default()
                            });
                        }
                        ProcessStatus::Unknown => {
                            return Ok(ToolOutput {
                                content: "unknown status".to_string(),
                                ..Default::default()
                            });
                        }
                    }
                } else {
                    return Ok(ToolOutput {
                        content: format!("process {pid} is not visible to this agent"),
                        status: ToolResultStatus::Failure,
                        ..Default::default()
                    });
                }

                if start.elapsed().as_millis() as u64 >= timeout_ms {
                    return Ok(ToolOutput {
                        content: format!(
                            "timeout: process {pid} still running after {timeout_ms}ms"
                        ),
                        status: ToolResultStatus::Failure,
                        ..Default::default()
                    });
                }

                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        })
    }
}

// ── ReadProcessOutputTool ─────────────────────────────────────────────────────

/// LLM tool: read captured stdout/stderr from a non-interactive process.
///
/// Only works for processes that were spawned with output capture (via
/// [`spawn_captured`](crate::platform::linux::namespace::NamespaceContainer::spawn_captured)).
pub struct ReadProcessOutputTool {
    scope: ProcessVisibilityScope,
    schema: OnceLock<ToolSchema>,
}

impl ReadProcessOutputTool {
    /// Create a new `read_process_output` tool.
    pub const fn new(scope: ProcessVisibilityScope) -> Self {
        Self {
            scope,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ReadProcessOutputTool {
    fn name(&self) -> &'static str {
        "read_process_output"
    }

    fn description(&self) -> &'static str {
        "Read captured stdout or stderr from a process. Only available for \
         non-interactive processes spawned with output capture. For combined \
         mode, 'stdout' returns the combined stream. Returns the output content \
         as a string."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "read_process_output".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pid": {
                        "type": "integer",
                        "description": "The process ID whose output to read.",
                        "minimum": 1
                    },
                    "stream": {
                        "type": "string",
                        "description": "Which stream to read: 'stdout' (or combined output), 'stderr'.",
                        "enum": ["stdout", "stderr"],
                        "default": "stdout"
                    }
                },
                "required": ["pid"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let pid = parse_pid(&input)?;
            let stream = input["stream"].as_str().unwrap_or("stdout");

            let (_label, record) =
                self.scope.find(pid).await.ok_or_else(|| {
                    tool_err(format!("process {pid} is not visible to this agent"))
                })?;

            let captured = record
                .output
                .as_ref()
                .ok_or_else(|| tool_err("no captured output for this process"))?;

            let content = match stream {
                "stderr" => captured
                    .read_stderr()
                    .map_err(|e| tool_err(e.to_string()))?
                    .unwrap_or_else(|| "(streams are combined — use 'stdout' to read)".to_string()),
                _ => captured
                    .read_stdout()
                    .map_err(|e| tool_err(e.to_string()))?,
            };

            Ok(ToolOutput {
                content,
                ..Default::default()
            })
        })
    }
}

// ── platform signal helpers ────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn send_signal_to_pid(pid: u32, signal_name: &str) -> Result<(), SynwireError> {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    let sig = parse_signal(signal_name)?;
    kill(Pid::from_raw(pid as i32), sig)
        .map_err(|e| tool_err(format!("failed to send {signal_name} to pid {pid}: {e}")))
}

#[cfg(target_os = "macos")]
fn send_signal_to_pid(pid: u32, signal_name: &str) -> Result<(), SynwireError> {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    let sig = parse_signal(signal_name)?;
    kill(Pid::from_raw(pid as i32), sig)
        .map_err(|e| tool_err(format!("failed to send {signal_name} to pid {pid}: {e}")))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn send_signal_to_pid(_pid: u32, _signal_name: &str) -> Result<(), SynwireError> {
    Err(tool_err("signal sending not supported on this platform"))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn parse_signal(name: &str) -> Result<nix::sys::signal::Signal, SynwireError> {
    use nix::sys::signal::Signal;
    match name {
        "SIGTERM" => Ok(Signal::SIGTERM),
        "SIGKILL" => Ok(Signal::SIGKILL),
        "SIGINT" => Ok(Signal::SIGINT),
        "SIGHUP" => Ok(Signal::SIGHUP),
        "SIGSTOP" => Ok(Signal::SIGSTOP),
        "SIGCONT" => Ok(Signal::SIGCONT),
        other => Err(validation_err(format!("unknown signal: {other}"))),
    }
}

fn signal_name_to_number(name: &str) -> i32 {
    match name {
        "SIGTERM" => 15,
        "SIGKILL" => 9,
        "SIGINT" => 2,
        "SIGHUP" => 1,
        "SIGSTOP" => 19,
        "SIGCONT" => 18,
        _ => 15,
    }
}

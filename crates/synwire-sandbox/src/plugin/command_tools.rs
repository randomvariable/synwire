#![allow(
    clippy::significant_drop_tightening,
    clippy::option_if_let_else,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]
//! LLM-callable tools for spawning and interacting with sandboxed commands.
//!
//! These tools are the primary interface for an LLM agent to execute commands:
//!
//! - [`RunCommandTool`] — spawn a command, optionally wait for completion
//! - [`OpenShellTool`] — open an interactive PTY session for HITL
//! - [`ShellWriteTool`] — send input to a PTY session
//! - [`ShellReadTool`] — read available output from a PTY session
//! - [`ShellExpectTool`] — wait for a regex pattern (like `expect(1)`)
//! - [`ShellExpectCasesTool`] — wait for one of N patterns (switch/case)
//! - [`ShellBatchTool`] — run a send/expect sequence in one call
//! - [`ShellSignalTool`] — send an OS signal to a shell session
//!
//! # goexpect compatibility
//!
//! All expect operations are backed by [`expectrl`], providing:
//! - Full regex matching with capture groups
//! - Multi-pattern switch/case via [`expectrl::Any`]
//! - Configurable timeouts (0 = dump buffer, default = 30s)
//! - Cross-platform PTY support (Linux + macOS)

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use serde_json::{Value, json};

use synwire_core::BoxFuture;
use synwire_core::error::{SynwireError, ToolError};
use synwire_core::tools::{Tool, ToolOutput, ToolResultStatus, ToolSchema};

use crate::output::OutputMode;
use crate::process_registry::{ProcessRecord, monitor_child};

use super::context::SandboxContext;
use super::expect_engine::{
    BatchStep, BatchStepResult, CaseTag, ExpectCase, expand_captures, extract_matches,
    session_from_fd,
};

// ── helpers ──────────────────────────────────────────────────────────────────

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

// ── RunCommandTool ──────────────────────────────────────────────────────────

/// LLM tool: run a command inside the sandbox.
///
/// Two modes:
/// - **Oneshot** (`wait: true`, default): blocks until the command exits, returns
///   exit code + stdout + stderr in a single response.
/// - **Background** (`wait: false`): returns immediately with the PID. Use
///   `wait_for_process` and `read_process_output` to poll status and read output.
pub struct RunCommandTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl RunCommandTool {
    /// Create a new `run_command` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for RunCommandTool {
    fn name(&self) -> &'static str {
        "run_command"
    }

    fn description(&self) -> &'static str {
        "Run a command inside the sandbox. By default waits for completion \
         and returns the exit code, stdout, and stderr. Set wait=false to run in \
         background and get a PID back — then use wait_for_process and \
         read_process_output to check status and read output."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "run_command".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to execute (e.g., 'cargo', 'terraform')."
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Command arguments.",
                        "default": []
                    },
                    "wait": {
                        "type": "boolean",
                        "description": "If true (default), wait for completion. If false, return PID for background monitoring.",
                        "default": true
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Max seconds to wait (only when wait=true). Default: 30.",
                        "default": 30,
                        "minimum": 1,
                        "maximum": 3600
                    }
                },
                "required": ["command"]
            }),
        })
    }

    #[cfg(target_os = "linux")]
    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            use crate::platform::linux::namespace::NamespaceContainer;

            let command = input["command"]
                .as_str()
                .ok_or_else(|| validation_err("'command' is required"))?;

            let args: Vec<String> = input["args"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let wait = input["wait"].as_bool().unwrap_or(true);
            let timeout_secs = input["timeout_secs"].as_u64().unwrap_or(30);

            let cc = NamespaceContainer::build_config(&self.ctx.config, command, args.clone());

            let capture = self
                .ctx
                .container
                .spawn_captured(&cc, OutputMode::Separate)
                .map_err(|e| tool_err(format!("spawn failed: {e}")))?;

            let pid = capture
                .child
                .id()
                .ok_or_else(|| tool_err("child has no PID"))?;

            let mut record = ProcessRecord::new(pid, command, args);
            record.output = Some(Arc::clone(&capture.output));
            {
                let mut reg = self.ctx.registry.write().await;
                reg.insert(record).map_err(|e| tool_err(e.to_string()))?;
            }

            if wait {
                let mut child = capture.child;
                let status =
                    tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await;

                let (exit_code, timed_out) = match status {
                    Ok(Ok(s)) => (s.code().unwrap_or(-1), false),
                    Ok(Err(e)) => return Err(tool_err(format!("wait failed: {e}"))),
                    Err(_) => {
                        let _ = child.kill().await;
                        (-1, true)
                    }
                };

                {
                    let mut reg = self.ctx.registry.write().await;
                    if timed_out {
                        reg.mark_signaled(pid, 9);
                    } else {
                        reg.mark_exited(pid, exit_code);
                    }
                }

                let stdout = capture
                    .output
                    .read_stdout()
                    .map_err(|e| tool_err(e.to_string()))?;
                let stderr = capture
                    .output
                    .read_stderr()
                    .map_err(|e| tool_err(e.to_string()))?
                    .unwrap_or_default();

                let result = json!({
                    "pid": pid, "exit_code": exit_code, "timed_out": timed_out,
                    "stdout": stdout, "stderr": stderr,
                });

                Ok(ToolOutput {
                    content: serde_json::to_string_pretty(&result)
                        .map_err(|e| tool_err(e.to_string()))?,
                    status: if exit_code == 0 {
                        ToolResultStatus::Success
                    } else {
                        ToolResultStatus::Failure
                    },
                    ..Default::default()
                })
            } else {
                monitor_child(capture.child, pid, Arc::clone(&self.ctx.registry));
                let result = json!({
                    "pid": pid, "status": "running",
                    "hint": "Use wait_for_process to block until exit, or read_process_output to read partial output."
                });
                Ok(ToolOutput {
                    content: serde_json::to_string_pretty(&result)
                        .map_err(|e| tool_err(e.to_string()))?,
                    ..Default::default()
                })
            }
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn invoke(&self, _input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async { Err(tool_err("run_command is only supported on Linux")) })
    }
}

// ── OpenShellTool ───────────────────────────────────────────────────────────

/// LLM tool: open an interactive PTY shell session backed by expectrl.
pub struct OpenShellTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl OpenShellTool {
    /// Create a new `open_shell` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for OpenShellTool {
    fn name(&self) -> &'static str {
        "open_shell"
    }

    fn description(&self) -> &'static str {
        "Open an interactive shell session inside the sandbox. Returns a session_id. \
         Use shell_expect, shell_write, shell_read, shell_expect_cases, or shell_batch \
         to interact. For human-in-the-loop scenarios where the user needs to type \
         (e.g., confirming terraform apply, entering credentials)."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "open_shell".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "shell": { "type": "string", "description": "Shell to launch.", "default": "/bin/sh" }
                },
                "required": []
            }),
        })
    }

    #[cfg(target_os = "linux")]
    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            use crate::platform::linux::namespace::NamespaceContainer;

            let shell = input["shell"].as_str().unwrap_or("/bin/sh");
            let cc = NamespaceContainer::build_config(&self.ctx.config, shell, vec![]);

            let pty_session = self
                .ctx
                .container
                .spawn_interactive(&cc)
                .map_err(|e| tool_err(format!("open_shell failed: {e}")))?;

            // Wrap the PTY controller fd in an expectrl session
            let expect_session = session_from_fd(pty_session.controller)
                .map_err(|e| tool_err(format!("create expect session: {e}")))?;

            let session_id = uuid::Uuid::new_v4().to_string();

            {
                let mut sessions = self.ctx.sessions.lock().await;
                let _ = sessions.insert(session_id.clone(), expect_session);
            }
            {
                let mut children = self.ctx.session_children.lock().await;
                let _ = children.insert(session_id.clone(), pty_session.child);
            }

            let result = json!({
                "session_id": session_id,
                "shell": shell,
                "hint": "Use shell_expect to wait for prompts, shell_write to send input, shell_batch for sequences."
            });
            Ok(ToolOutput {
                content: serde_json::to_string_pretty(&result)
                    .map_err(|e| tool_err(e.to_string()))?,
                ..Default::default()
            })
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn invoke(&self, _input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async { Err(tool_err("open_shell is only supported on Linux")) })
    }
}

// ── ShellWriteTool ──────────────────────────────────────────────────────────

/// LLM tool: send input to a PTY session via expectrl.
pub struct ShellWriteTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl ShellWriteTool {
    /// Create a new `shell_write` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ShellWriteTool {
    fn name(&self) -> &'static str {
        "shell_write"
    }

    fn description(&self) -> &'static str {
        "Send input text to an interactive shell session. Use \\n for Enter."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "shell_write".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from open_shell." },
                    "input": { "type": "string", "description": "Text to send. Use \\n for Enter." }
                },
                "required": ["session_id", "input"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            use expectrl::Expect;

            let session_id = input["session_id"]
                .as_str()
                .ok_or_else(|| validation_err("'session_id' is required"))?;
            let text = input["input"]
                .as_str()
                .ok_or_else(|| validation_err("'input' is required"))?;

            let mut sessions = self.ctx.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| tool_err(format!("session '{session_id}' not found")))?;

            session
                .send(text)
                .map_err(|e| tool_err(format!("send failed: {e}")))?;

            Ok(ToolOutput {
                content: format!("sent {} bytes to session {session_id}", text.len()),
                ..Default::default()
            })
        })
    }
}

// ── ShellReadTool ───────────────────────────────────────────────────────────

/// LLM tool: non-blocking read of available PTY output.
pub struct ShellReadTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl ShellReadTool {
    /// Create a new `shell_read` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ShellReadTool {
    fn name(&self) -> &'static str {
        "shell_read"
    }

    fn description(&self) -> &'static str {
        "Read available output from a shell session. Non-blocking — returns \
         empty string if no output is available yet."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "shell_read".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from open_shell." }
                },
                "required": ["session_id"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            use expectrl::Expect;

            let session_id = input["session_id"]
                .as_str()
                .ok_or_else(|| validation_err("'session_id' is required"))?;

            let mut sessions = self.ctx.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| tool_err(format!("session '{session_id}' not found")))?;

            // Use expectrl's check with a regex that matches anything,
            // effectively reading whatever is available.
            let timeout_backup = Duration::from_millis(0);
            session.set_expect_timeout(Some(timeout_backup));

            // Try to read whatever is in the buffer — Eof or timeout both mean "nothing new"
            let content = match session.expect(expectrl::Eof) {
                Ok(captures) => {
                    let before = captures.before();
                    String::from_utf8_lossy(before).into_owned()
                }
                Err(_) => {
                    // No data or timeout — check the buffer directly
                    String::new()
                }
            };

            // Restore a reasonable timeout
            session.set_expect_timeout(Some(Duration::from_secs(30)));

            Ok(ToolOutput {
                content,
                ..Default::default()
            })
        })
    }
}

// ── ShellExpectTool ─────────────────────────────────────────────────────────

/// LLM tool: wait for a regex pattern in PTY output.
///
/// Returns all accumulated output up to the match, plus captured groups.
pub struct ShellExpectTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl ShellExpectTool {
    /// Create a new `shell_expect` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ShellExpectTool {
    fn name(&self) -> &'static str {
        "shell_expect"
    }

    fn description(&self) -> &'static str {
        "Wait for a regex pattern in the shell output. Returns all output \
         captured up to the match, plus captured groups from the regex. \
         Use this to detect prompts (e.g., 'Enter a value:', 'password:', \
         '[y/N]') before deciding to respond or hand off to the user."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "shell_expect".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from open_shell." },
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to match. Supports capture groups. Examples: 'Enter a value:', 'version (\\d+\\.\\d+)', '\\$\\s*$'."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Max seconds to wait. 0 = check buffer only. Default: 30.",
                        "default": 30, "minimum": 0, "maximum": 300
                    }
                },
                "required": ["session_id", "pattern"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            use expectrl::Expect;

            let session_id = input["session_id"]
                .as_str()
                .ok_or_else(|| validation_err("'session_id' is required"))?;
            let pattern = input["pattern"]
                .as_str()
                .ok_or_else(|| validation_err("'pattern' is required"))?;
            let timeout_secs = input["timeout_secs"].as_u64().unwrap_or(30);

            let re = expectrl::Regex(pattern.to_string());

            let mut sessions = self.ctx.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| tool_err(format!("session '{session_id}' not found")))?;

            session.set_expect_timeout(Some(Duration::from_secs(timeout_secs)));

            match session.expect(re) {
                Ok(captures) => {
                    let before = String::from_utf8_lossy(captures.before()).into_owned();
                    let matched_groups = extract_matches(&captures);

                    let mut output = before;
                    if let Some(full_match) = matched_groups.first() {
                        output.push_str(full_match);
                    }

                    let result = json!({
                        "matched": true,
                        "pattern": pattern,
                        "output": output,
                        "captures": matched_groups,
                    });
                    Ok(ToolOutput {
                        content: serde_json::to_string_pretty(&result)
                            .map_err(|e| tool_err(e.to_string()))?,
                        ..Default::default()
                    })
                }
                Err(e) => {
                    let result = json!({
                        "matched": false,
                        "pattern": pattern,
                        "output": "",
                        "captures": [],
                        "reason": e.to_string(),
                    });
                    Ok(ToolOutput {
                        content: serde_json::to_string_pretty(&result)
                            .map_err(|e| tool_err(e.to_string()))?,
                        status: ToolResultStatus::Failure,
                        ..Default::default()
                    })
                }
            }
        })
    }
}

// ── ShellExpectCasesTool ────────────────────────────────────────────────────

/// LLM tool: wait for one of several regex patterns (switch/case).
///
/// Maps to goexpect's `ExpectSwitchCase`. Returns which case matched first,
/// captured groups, and optionally auto-sends a response.
pub struct ShellExpectCasesTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl ShellExpectCasesTool {
    /// Create a new `shell_expect_cases` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ShellExpectCasesTool {
    fn name(&self) -> &'static str {
        "shell_expect_cases"
    }

    fn description(&self) -> &'static str {
        "Wait for one of several regex patterns (switch/case). Returns which \
         case matched first, plus captures. Each case has a tag ('ok', 'fail', \
         'continue', 'needs_user') and an optional auto-response. Use this \
         when the CLI might show different prompts (success, error, auth prompt)."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "shell_expect_cases".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from open_shell." },
                    "cases": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "pattern": { "type": "string", "description": "Regex pattern." },
                                "tag": {
                                    "type": "string",
                                    "enum": ["ok", "fail", "continue", "needs_user", "next"],
                                    "description": "Flow control tag."
                                },
                                "respond": { "type": "string", "description": "Auto-response to send if matched. $1/$2 for captures." },
                                "label": { "type": "string", "description": "Human-readable label." }
                            },
                            "required": ["pattern", "tag"]
                        },
                        "description": "Cases to match. First match wins."
                    },
                    "timeout_secs": {
                        "type": "integer", "description": "Max seconds to wait. Default: 30.",
                        "default": 30, "minimum": 0, "maximum": 300
                    }
                },
                "required": ["session_id", "cases"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            use expectrl::Expect;

            let session_id = input["session_id"]
                .as_str()
                .ok_or_else(|| validation_err("'session_id' is required"))?;
            let timeout_secs = input["timeout_secs"].as_u64().unwrap_or(30);

            let cases: Vec<ExpectCase> = serde_json::from_value(input["cases"].clone())
                .map_err(|e| validation_err(format!("invalid 'cases': {e}")))?;

            if cases.is_empty() {
                return Err(validation_err("'cases' must not be empty"));
            }

            let mut sessions = self.ctx.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| tool_err(format!("session '{session_id}' not found")))?;

            session.set_expect_timeout(Some(Duration::from_secs(timeout_secs)));

            // Build expectrl needles inside the lock scope (dyn Needle is !Send)
            let needles: Vec<Box<dyn expectrl::Needle>> = cases
                .iter()
                .map(|c| -> Box<dyn expectrl::Needle> {
                    Box::new(expectrl::Regex(c.pattern.clone()))
                })
                .collect();
            let any = expectrl::Any::boxed(needles);

            match session.expect(any) {
                Ok(captures) => {
                    let before = String::from_utf8_lossy(captures.before()).into_owned();
                    let groups = extract_matches(&captures);

                    // Determine which case matched by checking each pattern
                    let full_match = groups.first().cloned().unwrap_or_default();
                    let mut matched_idx = None;
                    for (i, case) in cases.iter().enumerate() {
                        if let std::result::Result::Ok(re) = regex::Regex::new(&case.pattern) {
                            if re.is_match(&full_match) {
                                matched_idx = Some(i);
                                break;
                            }
                        }
                    }

                    let idx = matched_idx.unwrap_or(0);
                    let matched_case = &cases[idx];

                    // Auto-respond if configured
                    if let Some(ref respond) = matched_case.respond {
                        let expanded = expand_captures(respond, &groups);
                        let _send_result = session.send(&expanded);
                    }

                    let mut output = before;
                    output.push_str(&full_match);

                    let result = json!({
                        "matched": true,
                        "matched_case": idx,
                        "tag": matched_case.tag,
                        "label": matched_case.label,
                        "output": output,
                        "captures": groups,
                    });

                    let status = match matched_case.tag {
                        CaseTag::Fail => ToolResultStatus::Failure,
                        _ => ToolResultStatus::Success,
                    };

                    Ok(ToolOutput {
                        content: serde_json::to_string_pretty(&result)
                            .map_err(|e| tool_err(e.to_string()))?,
                        status,
                        ..Default::default()
                    })
                }
                Err(e) => {
                    let result = json!({
                        "matched": false,
                        "output": "",
                        "reason": e.to_string(),
                    });
                    Ok(ToolOutput {
                        content: serde_json::to_string_pretty(&result)
                            .map_err(|e| tool_err(e.to_string()))?,
                        status: ToolResultStatus::Failure,
                        ..Default::default()
                    })
                }
            }
        })
    }
}

// ── ShellBatchTool ──────────────────────────────────────────────────────────

/// LLM tool: run a sequence of send/expect steps in one call.
///
/// Maps to goexpect's `ExpectBatch`. Stops on first failure.
pub struct ShellBatchTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl ShellBatchTool {
    /// Create a new `shell_batch` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ShellBatchTool {
    fn name(&self) -> &'static str {
        "shell_batch"
    }

    fn description(&self) -> &'static str {
        "Run a sequence of send/expect operations in one call. Each step is \
         either 'send' (write to PTY), 'expect' (wait for pattern), \
         'expect_cases' (wait for one of N patterns), or 'signal' (send OS signal). \
         Stops on first failure. Returns results for each completed step."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "shell_batch".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from open_shell." },
                    "steps": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["send", "expect", "expect_cases", "signal"] },
                                "input": { "type": "string", "description": "Text to send (for 'send' steps)." },
                                "pattern": { "type": "string", "description": "Regex pattern (for 'expect' steps)." },
                                "cases": { "type": "array", "description": "Cases (for 'expect_cases' steps)." },
                                "signal": { "type": "string", "description": "Signal name (for 'signal' steps)." },
                                "timeout_secs": { "type": "integer", "description": "Per-step timeout override." }
                            },
                            "required": ["type"]
                        }
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Default timeout for expect steps. Default: 30.",
                        "default": 30
                    }
                },
                "required": ["session_id", "steps"]
            }),
        })
    }

    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            use expectrl::Expect;

            let session_id = input["session_id"]
                .as_str()
                .ok_or_else(|| validation_err("'session_id' is required"))?;
            let default_timeout = input["timeout_secs"].as_u64().unwrap_or(30);

            let steps: Vec<BatchStep> = serde_json::from_value(input["steps"].clone())
                .map_err(|e| validation_err(format!("invalid 'steps': {e}")))?;

            let mut results: Vec<BatchStepResult> = Vec::new();

            let mut sessions = self.ctx.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| tool_err(format!("session '{session_id}' not found")))?;

            for (i, step) in steps.iter().enumerate() {
                let step_result = match step {
                    BatchStep::Send { input: text } => match session.send(text.as_str()) {
                        Ok(()) => BatchStepResult {
                            index: i,
                            step_type: "send".into(),
                            output: None,
                            captures: vec![],
                            matched_case: None,
                            tag: None,
                            label: None,
                            success: true,
                            error: None,
                        },
                        Err(e) => BatchStepResult {
                            index: i,
                            step_type: "send".into(),
                            output: None,
                            captures: vec![],
                            matched_case: None,
                            tag: None,
                            label: None,
                            success: false,
                            error: Some(e.to_string()),
                        },
                    },
                    BatchStep::Expect {
                        pattern,
                        timeout_secs,
                    } => {
                        let timeout = timeout_secs.unwrap_or(default_timeout);
                        session.set_expect_timeout(Some(Duration::from_secs(timeout)));

                        match session.expect(expectrl::Regex(pattern.clone())) {
                            Ok(captures) => {
                                let before =
                                    String::from_utf8_lossy(captures.before()).into_owned();
                                let groups = extract_matches(&captures);
                                let full = groups.first().cloned().unwrap_or_default();
                                BatchStepResult {
                                    index: i,
                                    step_type: "expect".into(),
                                    output: Some(format!("{before}{full}")),
                                    captures: groups,
                                    matched_case: None,
                                    tag: None,
                                    label: None,
                                    success: true,
                                    error: None,
                                }
                            }
                            Err(e) => BatchStepResult {
                                index: i,
                                step_type: "expect".into(),
                                output: None,
                                captures: vec![],
                                matched_case: None,
                                tag: None,
                                label: None,
                                success: false,
                                error: Some(e.to_string()),
                            },
                        }
                    }
                    BatchStep::ExpectCases {
                        cases,
                        timeout_secs,
                    } => {
                        let timeout = timeout_secs.unwrap_or(default_timeout);
                        session.set_expect_timeout(Some(Duration::from_secs(timeout)));

                        let needles: Vec<Box<dyn expectrl::Needle>> = cases
                            .iter()
                            .map(|c| -> Box<dyn expectrl::Needle> {
                                Box::new(expectrl::Regex(c.pattern.clone()))
                            })
                            .collect();
                        let any = expectrl::Any::boxed(needles);

                        match session.expect(any) {
                            Ok(captures) => {
                                let before =
                                    String::from_utf8_lossy(captures.before()).into_owned();
                                let groups = extract_matches(&captures);
                                let full = groups.first().cloned().unwrap_or_default();

                                let mut idx = 0;
                                for (j, case) in cases.iter().enumerate() {
                                    if let std::result::Result::Ok(re) =
                                        regex::Regex::new(&case.pattern)
                                    {
                                        if re.is_match(&full) {
                                            idx = j;
                                            break;
                                        }
                                    }
                                }

                                let matched_case = &cases[idx];
                                if let Some(ref respond) = matched_case.respond {
                                    let expanded = expand_captures(respond, &groups);
                                    let _r = session.send(&expanded);
                                }

                                let success = matched_case.tag != CaseTag::Fail;
                                BatchStepResult {
                                    index: i,
                                    step_type: "expect_cases".into(),
                                    output: Some(format!("{before}{full}")),
                                    captures: groups,
                                    matched_case: Some(idx),
                                    tag: Some(matched_case.tag.clone()),
                                    label: matched_case.label.clone(),
                                    success,
                                    error: None,
                                }
                            }
                            Err(e) => BatchStepResult {
                                index: i,
                                step_type: "expect_cases".into(),
                                output: None,
                                captures: vec![],
                                matched_case: None,
                                tag: None,
                                label: None,
                                success: false,
                                error: Some(e.to_string()),
                            },
                        }
                    }
                    BatchStep::Signal { signal } => {
                        // Signals are handled via the child process, not the PTY session
                        BatchStepResult {
                            index: i,
                            step_type: "signal".into(),
                            output: None,
                            captures: vec![],
                            matched_case: None,
                            tag: None,
                            label: None,
                            success: false,
                            error: Some(format!("use shell_signal for signal '{signal}'")),
                        }
                    }
                };

                let failed = !step_result.success;
                results.push(step_result);
                if failed {
                    break;
                }
            }

            let all_ok = results.iter().all(|r| r.success);
            let result =
                json!({ "steps": results, "completed": results.len(), "total": steps.len() });

            Ok(ToolOutput {
                content: serde_json::to_string_pretty(&result)
                    .map_err(|e| tool_err(e.to_string()))?,
                status: if all_ok {
                    ToolResultStatus::Success
                } else {
                    ToolResultStatus::Failure
                },
                ..Default::default()
            })
        })
    }
}

// ── ShellSignalTool ─────────────────────────────────────────────────────────

/// LLM tool: send an OS signal to a shell session's process.
pub struct ShellSignalTool {
    ctx: Arc<SandboxContext>,
    schema: OnceLock<ToolSchema>,
}

impl ShellSignalTool {
    /// Create a new `shell_signal` tool.
    pub const fn new(ctx: Arc<SandboxContext>) -> Self {
        Self {
            ctx,
            schema: OnceLock::new(),
        }
    }
}

impl Tool for ShellSignalTool {
    fn name(&self) -> &'static str {
        "shell_signal"
    }

    fn description(&self) -> &'static str {
        "Send an OS signal to a shell session's process. Use SIGINT (Ctrl-C) \
         to cancel a running command, SIGTERM to terminate gracefully."
    }

    fn schema(&self) -> &ToolSchema {
        self.schema.get_or_init(|| ToolSchema {
            name: "shell_signal".into(),
            description: self.description().into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from open_shell." },
                    "signal": {
                        "type": "string",
                        "enum": ["SIGINT", "SIGTERM", "SIGKILL", "SIGHUP", "SIGSTOP", "SIGCONT"],
                        "description": "Signal to send. Default: SIGINT.",
                        "default": "SIGINT"
                    }
                },
                "required": ["session_id"]
            }),
        })
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn invoke(&self, input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let session_id = input["session_id"]
                .as_str()
                .ok_or_else(|| validation_err("'session_id' is required"))?;
            let signal_name = input["signal"].as_str().unwrap_or("SIGINT");

            let children = self.ctx.session_children.lock().await;
            let child = children
                .get(session_id)
                .ok_or_else(|| tool_err(format!("session '{session_id}' not found")))?;

            let pid = child
                .id()
                .ok_or_else(|| tool_err("session process has no PID"))?;

            let sig = match signal_name {
                "SIGINT" => nix::sys::signal::Signal::SIGINT,
                "SIGTERM" => nix::sys::signal::Signal::SIGTERM,
                "SIGKILL" => nix::sys::signal::Signal::SIGKILL,
                "SIGHUP" => nix::sys::signal::Signal::SIGHUP,
                "SIGSTOP" => nix::sys::signal::Signal::SIGSTOP,
                "SIGCONT" => nix::sys::signal::Signal::SIGCONT,
                other => return Err(validation_err(format!("unknown signal: {other}"))),
            };

            nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), sig)
                .map_err(|e| tool_err(format!("kill({pid}, {signal_name}): {e}")))?;

            Ok(ToolOutput {
                content: format!("sent {signal_name} to session {session_id} (pid {pid})"),
                ..Default::default()
            })
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn invoke(&self, _input: Value) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async { Err(tool_err("shell_signal is only supported on Unix")) })
    }
}

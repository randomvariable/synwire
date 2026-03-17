//! Expect engine backed by [`expectrl`] — implements goexpect-equivalent semantics.
//!
//! This module provides:
//!
//! - [`PtyStream`]: a wrapper around `OwnedFd` that implements `Read + Write +
//!   NonBlocking` for use with expectrl's `Session`.
//! - [`StubProcess`]: a `Healthcheck` impl for processes managed externally
//!   (by runc/runsc) rather than spawned by expectrl.
//! - [`ExpectCase`], [`BatchStep`], etc.: types for the LLM tool layer.
//!
//! # goexpect compatibility
//!
//! | goexpect | expectrl | Our tool |
//! |----------|----------|----------|
//! | `Expect(re, timeout)` | `session.expect(Regex("..."))` | `shell_expect` |
//! | `ExpectSwitchCase([]Caser)` | `session.expect(Any::boxed(..))` | `shell_expect_cases` |
//! | `ExpectBatch([]Batcher)` | Sequential send+expect | `shell_batch` |
//! | `Send(string)` | `session.send(string)` | `shell_write` |
//! | `SendSignal(sig)` | External kill | `shell_signal` |
//! | Tags: OK/Fail/Continue/Next | [`CaseTag`] | flow control |
//!
//! # macOS compatibility
//!
//! `expectrl` handles platform differences internally. On macOS it uses
//! `posix_openpt` / `grantpt` / `unlockpt` for PTY allocation and
//! `fcntl(O_NONBLOCK)` for non-blocking I/O — no Linux-specific APIs needed
//! in this module.

use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};

use serde::{Deserialize, Serialize};

// ── PtyStream ────────────────────────────────────────────────────────────────

/// A stream wrapper around an `OwnedFd` that implements the traits expectrl
/// needs: `Read + Write + NonBlocking`.
///
/// Used to wrap the PTY controller fd received from the OCI runtime's console
/// socket. Works on both Linux and macOS since it only uses POSIX `read`,
/// `write`, and `fcntl`.
#[derive(Debug)]
pub struct PtyStream {
    fd: OwnedFd,
}

impl PtyStream {
    /// Wrap an owned PTY controller file descriptor.
    pub const fn new(fd: OwnedFd) -> Self {
        Self { fd }
    }

    /// Access the underlying fd.
    pub const fn as_fd(&self) -> &OwnedFd {
        &self.fd
    }
}

impl Read for PtyStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        nix::unistd::read(self.fd.as_raw_fd(), buf).map_err(io::Error::from)
    }
}

impl Write for PtyStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        nix::unistd::write(&self.fd, buf).map_err(io::Error::from)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl expectrl::process::NonBlocking for PtyStream {
    fn set_blocking(&mut self, on: bool) -> io::Result<()> {
        let raw_fd = self.fd.as_raw_fd();
        let flags =
            nix::fcntl::fcntl(raw_fd, nix::fcntl::FcntlArg::F_GETFL).map_err(io::Error::from)?;
        let mut oflags = nix::fcntl::OFlag::from_bits_truncate(flags);
        if on {
            oflags.remove(nix::fcntl::OFlag::O_NONBLOCK);
        } else {
            oflags.insert(nix::fcntl::OFlag::O_NONBLOCK);
        }
        let _rc = nix::fcntl::fcntl(raw_fd, nix::fcntl::FcntlArg::F_SETFL(oflags))
            .map_err(io::Error::from)?;
        Ok(())
    }
}

// ── StubProcess ──────────────────────────────────────────────────────────────

/// A stub process for use with expectrl's `Session` when the actual process
/// is managed externally (by runc/runsc).
///
/// `is_alive()` always returns `true` — the real liveness check is done via
/// the `ProcessRegistry` and the OCI runtime lifecycle.
#[derive(Debug)]
pub struct StubProcess;

impl expectrl::process::Healthcheck for StubProcess {
    type Status = bool;

    fn get_status(&self) -> io::Result<Self::Status> {
        Ok(true)
    }

    fn is_alive(&self) -> io::Result<bool> {
        Ok(true)
    }
}

// ── Types for LLM tool layer ─────────────────────────────────────────────────

/// Flow control tag for expect cases (maps to goexpect's `Tag`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CaseTag {
    /// Match accepted — stop matching and return success.
    #[serde(rename = "ok")]
    Ok,
    /// Match indicates failure — stop and return error.
    #[serde(rename = "fail")]
    Fail,
    /// Match found but keep trying — retry from the current buffer position.
    #[serde(rename = "continue")]
    Continue,
    /// Skip to the next batch step without consuming the match.
    #[serde(rename = "next")]
    Next,
    /// Requires human intervention — hand off to user.
    #[serde(rename = "needs_user")]
    NeedsUser,
}

/// A single case in a switch/case expect operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectCase {
    /// Regex pattern to match.
    pub pattern: String,
    /// Flow control tag when this case matches.
    pub tag: CaseTag,
    /// Optional auto-response to send when this case matches.
    /// Supports `$1`, `$2` etc. for captured group substitution.
    #[serde(default)]
    pub respond: Option<String>,
    /// Human-readable label for the case (returned to the LLM).
    #[serde(default)]
    pub label: Option<String>,
}

/// A single step in a batch sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum BatchStep {
    /// Send text to the PTY.
    #[serde(rename = "send")]
    Send {
        /// Text to send (use `\n` for Enter).
        input: String,
    },
    /// Wait for a single regex pattern.
    #[serde(rename = "expect")]
    Expect {
        /// Regex pattern to match.
        pattern: String,
        /// Per-step timeout override (seconds).
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
    /// Wait for one of several patterns (switch/case).
    #[serde(rename = "expect_cases")]
    ExpectCases {
        /// Cases to match against.
        cases: Vec<ExpectCase>,
        /// Per-step timeout override (seconds).
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
    /// Send an OS signal to the session's process.
    #[serde(rename = "signal")]
    Signal {
        /// Signal name (e.g., "SIGINT", "SIGTERM").
        signal: String,
    },
}

/// Result of a single batch step.
#[derive(Debug, Clone, Serialize)]
pub struct BatchStepResult {
    /// Step index (0-based).
    pub index: usize,
    /// The step type that was executed.
    pub step_type: String,
    /// Output captured during this step (for expect steps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Captured regex groups (for expect steps).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub captures: Vec<String>,
    /// Which case matched (for `expect_cases` steps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_case: Option<usize>,
    /// Tag of the matched case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<CaseTag>,
    /// Label of the matched case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether this step succeeded.
    pub success: bool,
    /// Error message if the step failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Substitute `$1`, `$2`, etc. in `template` with captured groups.
pub fn expand_captures(template: &str, captures: &[String]) -> String {
    let mut result = template.to_string();
    for (i, cap) in captures.iter().enumerate() {
        let placeholder = format!("${i}");
        result = result.replace(&placeholder, cap);
    }
    result
}

/// Extract captures from expectrl's `Captures` into a `Vec<String>`.
pub fn extract_matches(captures: &expectrl::Captures) -> Vec<String> {
    let mut result = Vec::new();
    for m in captures.matches() {
        result.push(String::from_utf8_lossy(m).into_owned());
    }
    result
}

/// Create an expectrl `Session` from a PTY controller fd.
///
/// The returned session is ready for `expect`, `send`, `check` etc.
/// The `StubProcess` reports the process as always alive — actual lifecycle
/// management is handled by the OCI runtime.
pub fn session_from_fd(fd: OwnedFd) -> io::Result<expectrl::Session<StubProcess, PtyStream>> {
    let stream = PtyStream::new(fd);
    expectrl::Session::new(StubProcess, stream)
}

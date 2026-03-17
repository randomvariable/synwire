//! Output capture for non-interactive sandbox processes.
//!
//! [`CapturedOutput`] stores stdout and stderr in a temporary directory that is
//! automatically removed when the last reference is dropped, giving Go-`defer`
//! lifecycle semantics.

use std::path::PathBuf;
use std::sync::Arc;

// ── OutputMode ───────────────────────────────────────────────────────────────

/// How stdout and stderr are captured for non-interactive processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputMode {
    /// stdout and stderr go to separate files (`stdout`, `stderr`).
    Separate,
    /// stdout and stderr interleave into a single file (`output`).
    Combined,
}

// ── CapturedOutput ───────────────────────────────────────────────────────────

/// Captured stdout/stderr from a non-interactive sandbox process.
///
/// Wraps a [`tempfile::TempDir`] that is automatically removed when the last
/// `Arc<CapturedOutput>` is dropped (either from the [`ProcessCapture`] handle
/// or from the [`ProcessRecord`](crate::ProcessRecord) in the registry).
#[derive(Debug)]
pub struct CapturedOutput {
    /// Temporary directory that owns the output files.
    dir: tempfile::TempDir,
    /// How stdout and stderr are stored.
    mode: OutputMode,
}

impl CapturedOutput {
    /// Allocate a new output capture directory.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if the directory cannot be created.
    pub fn new(mode: OutputMode) -> std::io::Result<Self> {
        Ok(Self {
            dir: tempfile::TempDir::with_prefix("synwire-")?,
            mode,
        })
    }

    /// Path to the stdout output file (or combined output file).
    #[must_use]
    pub fn stdout_path(&self) -> PathBuf {
        match self.mode {
            OutputMode::Combined => self.dir.path().join("output"),
            OutputMode::Separate => self.dir.path().join("stdout"),
        }
    }

    /// Path to the stderr output file, or `None` when streams are combined.
    #[must_use]
    pub fn stderr_path(&self) -> Option<PathBuf> {
        match self.mode {
            OutputMode::Separate => Some(self.dir.path().join("stderr")),
            OutputMode::Combined => None,
        }
    }

    /// Read captured stdout (or combined output) as a UTF-8 string.
    ///
    /// Returns an empty string if the file does not yet exist (process has not
    /// produced any output).
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if the file exists but cannot be read.
    pub fn read_stdout(&self) -> std::io::Result<String> {
        let path = self.stdout_path();
        if path.exists() {
            std::fs::read_to_string(path)
        } else {
            Ok(String::new())
        }
    }

    /// Read captured stderr as a UTF-8 string.
    ///
    /// Returns `None` when using [`OutputMode::Combined`] (use
    /// [`read_stdout`](Self::read_stdout) instead). Returns an empty string if
    /// the stderr file does not yet exist.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if the file exists but cannot be read.
    pub fn read_stderr(&self) -> std::io::Result<Option<String>> {
        match self.stderr_path() {
            Some(p) if p.exists() => std::fs::read_to_string(p).map(Some),
            Some(_) => Ok(Some(String::new())),
            None => Ok(None),
        }
    }

    /// The capture mode.
    #[must_use]
    pub const fn mode(&self) -> OutputMode {
        self.mode
    }
}

// ── ProcessCapture ───────────────────────────────────────────────────────────

/// Handle returned by [`NamespaceContainer::spawn_captured`](crate::platform::linux::namespace::NamespaceContainer::spawn_captured).
///
/// `output` is an [`Arc`]-wrapped [`CapturedOutput`]; the temp directory lives
/// as long as this handle or any [`ProcessRecord`](crate::ProcessRecord)
/// referring to the same `Arc` is alive. Pass `child` to
/// [`monitor_child`](crate::process_registry::monitor_child) for automatic
/// registry status updates when the process exits.
#[derive(Debug)]
pub struct ProcessCapture {
    /// Shared reference to the captured output directory.
    pub output: Arc<CapturedOutput>,
    /// The running child process.
    pub child: tokio::process::Child,
    /// OCI bundle directory — kept alive while the container runs.
    /// `None` for non-container processes.
    pub(crate) _bundle: Option<tempfile::TempDir>,
}

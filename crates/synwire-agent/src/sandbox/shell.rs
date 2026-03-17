//! Shell sandbox — extends `LocalProvider` with command execution.

use std::collections::HashMap;
use std::time::Duration;

use synwire_core::BoxFuture;
use synwire_core::vfs::error::VfsError;
use synwire_core::vfs::grep_options::GrepOptions;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{
    CpOptions, DirEntry, EditResult, ExecuteResponse, FileContent, GlobEntry, GrepMatch, LsOptions,
    RmOptions, TransferResult, VfsCapabilities, WriteResult,
};
use tokio::process::Command;
use tokio::time::timeout;

use crate::vfs::local::LocalProvider;

/// Maximum output bytes captured per stream before truncation.
const MAX_OUTPUT_BYTES: usize = 1024 * 1024; // 1 MiB

/// Shell that wraps `LocalProvider` and adds command execution.
pub struct Shell {
    fs: LocalProvider,
    env: HashMap<String, String>,
    default_timeout: Duration,
}

impl std::fmt::Debug for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shell").finish()
    }
}

impl Shell {
    /// Create a new shell rooted at `root`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`LocalProvider::new`].
    pub fn new(
        root: impl Into<std::path::PathBuf>,
        env: HashMap<String, String>,
        timeout_secs: u64,
    ) -> Result<Self, VfsError> {
        Ok(Self {
            fs: LocalProvider::new(root)?,
            env,
            default_timeout: Duration::from_secs(timeout_secs),
        })
    }

    /// Execute a command with optional timeout, returning truncated output.
    pub fn execute_cmd<'a>(
        &'a self,
        cmd: &'a str,
        args: &'a [String],
        timeout_override: Option<Duration>,
    ) -> BoxFuture<'a, Result<ExecuteResponse, VfsError>> {
        Box::pin(async move {
            let deadline = timeout_override.unwrap_or(self.default_timeout);
            let cwd = self.fs.pwd().await?;

            let child = Command::new(cmd)
                .args(args)
                .envs(&self.env)
                .current_dir(&cwd)
                .output();

            let output = timeout(deadline, child)
                .await
                .map_err(|_| VfsError::Timeout(format!("{cmd} timed out after {deadline:?}")))?
                .map_err(VfsError::Io)?;

            let stdout = truncate_string(
                String::from_utf8_lossy(&output.stdout).into_owned(),
                MAX_OUTPUT_BYTES,
            );
            let stderr = truncate_string(
                String::from_utf8_lossy(&output.stderr).into_owned(),
                MAX_OUTPUT_BYTES,
            );

            Ok(ExecuteResponse {
                exit_code: output.status.code().unwrap_or(-1),
                stdout,
                stderr,
            })
        })
    }
}

fn truncate_string(mut s: String, max: usize) -> String {
    const SUFFIX: &str = "\n[truncated]";
    if s.len() > max {
        let keep = max.saturating_sub(SUFFIX.len());
        // Walk back to a valid UTF-8 boundary.
        let mut boundary = keep;
        while boundary > 0 && !s.is_char_boundary(boundary) {
            boundary -= 1;
        }
        s.truncate(boundary);
        s.push_str(SUFFIX);
    }
    s
}

// Delegate all filesystem operations to the inner `LocalProvider`.
impl Vfs for Shell {
    fn ls(&self, path: &str, opts: LsOptions) -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>> {
        self.fs.ls(path, opts)
    }

    fn read(&self, path: &str) -> BoxFuture<'_, Result<FileContent, VfsError>> {
        self.fs.read(path)
    }

    fn write(&self, path: &str, content: &[u8]) -> BoxFuture<'_, Result<WriteResult, VfsError>> {
        self.fs.write(path, content)
    }

    fn edit(
        &self,
        path: &str,
        old: &str,
        new: &str,
    ) -> BoxFuture<'_, Result<EditResult, VfsError>> {
        self.fs.edit(path, old, new)
    }

    fn grep(
        &self,
        pattern: &str,
        opts: GrepOptions,
    ) -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>> {
        self.fs.grep(pattern, opts)
    }

    fn glob(&self, pattern: &str) -> BoxFuture<'_, Result<Vec<GlobEntry>, VfsError>> {
        self.fs.glob(pattern)
    }

    fn upload(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        self.fs.upload(from, to)
    }

    fn download(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        self.fs.download(from, to)
    }

    fn pwd(&self) -> BoxFuture<'_, Result<String, VfsError>> {
        self.fs.pwd()
    }

    fn cd(&self, path: &str) -> BoxFuture<'_, Result<(), VfsError>> {
        self.fs.cd(path)
    }

    fn rm(&self, path: &str, opts: RmOptions) -> BoxFuture<'_, Result<(), VfsError>> {
        self.fs.rm(path, opts)
    }

    fn cp(
        &self,
        from: &str,
        to: &str,
        opts: CpOptions,
    ) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        self.fs.cp(from, to, opts)
    }

    fn mv_file(&self, from: &str, to: &str) -> BoxFuture<'_, Result<TransferResult, VfsError>> {
        self.fs.mv_file(from, to)
    }

    fn capabilities(&self) -> VfsCapabilities {
        VfsCapabilities::all()
    }

    fn provider_name(&self) -> &'static str {
        "Shell"
    }
}

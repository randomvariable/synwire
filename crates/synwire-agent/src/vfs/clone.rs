//! Git repository clone and update operations.

use std::path::Path;
use std::process::Command;

/// Options controlling how a repository is cloned or updated.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CloneOptions {
    /// Remote URL to clone from.
    pub url: String,
    /// Shallow clone depth.  `None` performs a full clone.
    pub depth: Option<u32>,
    /// Branch, tag, or commit ref to check out after cloning.
    pub r#ref: Option<String>,
    /// Whether to trigger semantic indexing after a successful clone.
    pub index: bool,
}

impl CloneOptions {
    /// Construct minimal options with only the remote URL.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            depth: None,
            r#ref: None,
            index: false,
        }
    }
}

/// Errors produced by [`clone_or_update`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CloneError {
    /// A git operation failed.
    #[error("git error: {0}")]
    Git(String),
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(String),
}

/// Run a git command and map failures to [`CloneError`].
fn run_git(args: &[&str]) -> Result<(), CloneError> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| CloneError::Io(e.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(CloneError::Git(stderr.trim().to_owned()))
    }
}

/// Clone `options.url` into `dest`, or fetch + checkout if `dest` already exists.
///
/// When `dest` already contains a `.git` directory the function runs
/// `git fetch origin` followed by an optional `git checkout <ref>`.
/// Otherwise it performs a fresh `git clone`, honouring the optional
/// `depth` and `ref` fields in [`CloneOptions`].
///
/// # Errors
///
/// Returns [`CloneError::Git`] when a git command exits with a non-zero
/// status, carrying the stderr output. Returns [`CloneError::Io`] when the
/// git binary cannot be spawned.
pub fn clone_or_update(options: &CloneOptions, dest: &Path) -> Result<(), CloneError> {
    if dest.join(".git").is_dir() {
        // Existing repo — fetch and optionally checkout the requested ref.
        let dest_str = dest
            .to_str()
            .ok_or_else(|| CloneError::Io("destination path is not valid UTF-8".to_owned()))?;

        run_git(&["-C", dest_str, "fetch", "origin"])?;

        if let Some(git_ref) = &options.r#ref {
            run_git(&["-C", dest_str, "checkout", git_ref])?;
        }
    } else {
        // Fresh clone.
        let dest_str = dest
            .to_str()
            .ok_or_else(|| CloneError::Io("destination path is not valid UTF-8".to_owned()))?;

        let mut args: Vec<&str> = vec!["clone"];

        // Allocate the depth string outside the conditional so the borrow
        // lives long enough for the `args` slice.
        let depth_str;
        if let Some(depth) = options.depth {
            depth_str = depth.to_string();
            args.extend_from_slice(&["--depth", &depth_str]);
        }

        if let Some(git_ref) = &options.r#ref {
            args.extend_from_slice(&["--branch", git_ref]);
        }

        args.push(&options.url);
        args.push(dest_str);

        run_git(&args)?;
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn clone_invalid_url_returns_git_error() {
        let dir = tempdir().expect("tempdir");
        // Use a nonexistent dest sub-path so there is no `.git` dir and the
        // clone path is taken. The URL is intentionally bogus.
        let dest = dir.path().join("target");
        let opts = CloneOptions::new("https://invalid.example.test/no-such-repo.git");
        let err = clone_or_update(&opts, &dest);
        // Either a Git error (git is available but URL fails) or an IO error
        // (git binary not found) — but never "not yet implemented".
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(
            !msg.contains("not yet implemented"),
            "should no longer return stub error, got: {msg}"
        );
    }
}

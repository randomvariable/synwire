//! Agentic ignore file discovery and matching.
//!
//! LLM coding agents (Cursor, Copilot, Aider, Claude Code, Codeium, Tabby)
//! support ignore files that prevent the agent from reading or indexing
//! certain paths.  This module discovers those files by searching upward
//! from a given root directory — because ignore files in ancestor directories
//! should still apply to nested workspaces.
//!
//! All ignore files use gitignore syntax.

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};

/// File names recognised as agentic ignore controls.
///
/// Each file uses gitignore syntax.  Patterns are relative to the directory
/// containing the ignore file.
///
/// `.gitignore` is intentionally included — virtually all code tools respect
/// it, and files ignored by git are rarely useful to index or search.
pub const AGENTIC_IGNORE_FILES: &[&str] = &[
    ".gitignore",
    ".cursorignore",
    ".aiignore",
    ".claudeignore",
    ".aiderignore",
    ".codeiumignore",
    ".copilotignore",
    ".tabbyignore",
];

/// Combined matcher for agentic ignore files found by searching upward from a
/// root directory.
///
/// Patterns from files closer to the root take precedence over patterns from
/// ancestor directories, matching gitignore semantics.
#[derive(Default)]
pub struct AgenticIgnore {
    /// Matchers ordered from deepest (most specific) to shallowest (least
    /// specific).  Iteration returns the first definitive match.
    matchers: Vec<Gitignore>,
}

impl AgenticIgnore {
    /// Discover agentic ignore files by searching from `root` upward to the
    /// filesystem root.
    ///
    /// At each directory level, all recognised ignore file names are checked.
    /// The resulting matcher respects negation (`!` prefix) and directory-only
    /// patterns (trailing `/`) per gitignore specification.
    ///
    /// If `root` cannot be canonicalised, the raw path is used as-is.
    #[must_use]
    pub fn discover(root: &Path) -> Self {
        let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let mut matchers = Vec::new();
        let mut dir: Option<&Path> = Some(&canonical);

        while let Some(d) = dir {
            for &filename in AGENTIC_IGNORE_FILES {
                let ignore_path = d.join(filename);
                if ignore_path.is_file()
                    && let Some(gi) = load_ignore_file(d, &ignore_path)
                {
                    matchers.push(gi);
                }
            }
            dir = d.parent();
        }

        Self { matchers }
    }

    /// Return `true` if `path` should be excluded based on discovered ignore
    /// rules.
    ///
    /// Set `is_dir` to `true` when checking a directory path — this enables
    /// directory-only patterns (trailing `/` in gitignore syntax).
    #[must_use]
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        for gi in &self.matchers {
            match gi.matched(path, is_dir) {
                ignore::Match::Ignore(_) => return true,
                ignore::Match::Whitelist(_) => return false,
                ignore::Match::None => {}
            }
        }
        false
    }

    /// Return `true` if no ignore files were discovered.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.matchers.is_empty()
    }
}

/// Parse a single gitignore-format file.
fn load_ignore_file(root: &Path, path: &PathBuf) -> Option<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    // `add` returns `Some(Error)` on parse failure, `None` on success.
    if builder.add(path).is_some() {
        return None;
    }
    builder.build().ok()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn empty_when_no_ignore_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ai = AgenticIgnore::discover(dir.path());
        assert!(ai.is_empty());
        assert!(!ai.is_ignored(dir.path().join("foo.rs").as_path(), false));
    }

    #[test]
    fn respects_cursorignore() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join(".cursorignore"), "secret/\n*.env\n").expect("write");
        let ai = AgenticIgnore::discover(dir.path());
        assert!(!ai.is_empty());
        assert!(ai.is_ignored(&dir.path().join("secret"), true));
        assert!(ai.is_ignored(&dir.path().join(".env"), false));
        assert!(!ai.is_ignored(&dir.path().join("src/main.rs"), false));
    }

    #[test]
    fn respects_negation() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join(".aiignore"), "*.log\n!important.log\n").expect("write");
        let ai = AgenticIgnore::discover(dir.path());
        assert!(ai.is_ignored(&dir.path().join("debug.log"), false));
        assert!(!ai.is_ignored(&dir.path().join("important.log"), false));
    }

    #[test]
    fn traverses_upward() {
        let parent = tempfile::tempdir().expect("tempdir");
        let child = parent.path().join("project");
        fs::create_dir_all(&child).expect("mkdir");
        // Ignore file in parent directory
        fs::write(parent.path().join(".claudeignore"), "*.secret\n").expect("write");

        let ai = AgenticIgnore::discover(&child);
        assert!(ai.is_ignored(&child.join("api.secret"), false));
    }
}

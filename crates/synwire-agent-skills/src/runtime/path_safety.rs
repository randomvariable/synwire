//! Path traversal prevention utilities shared across skill runtimes.
//!
//! All VFS operations exposed to scripts are confined to a project root
//! directory. This module validates that a given path does not escape the root
//! via `..` segments or symlinks.

use std::path::{Path, PathBuf};

use crate::error::SkillError;

/// Resolve `requested` relative to `root`, ensuring the result is strictly
/// inside `root`.
///
/// Returns the resolved absolute path on success, or a [`SkillError::Runtime`]
/// if the path would escape the project root.
///
/// # Security
///
/// - Rejects paths containing `..` after joining with the root.
/// - Uses [`std::fs::canonicalize`] when the path exists to resolve symlinks.
/// - Rejects non-existent paths that contain `..` segments (cannot canonicalize
///   non-existent files, so we fall back to lexical checking).
#[allow(dead_code)]
pub fn safe_resolve(root: &Path, requested: &str) -> Result<PathBuf, SkillError> {
    let requested_path = Path::new(requested);

    // Build the candidate: if the request is absolute, strip the leading `/`
    // so it becomes relative to root.
    let candidate = if requested_path.is_absolute() {
        root.join(requested_path.strip_prefix("/").unwrap_or(requested_path))
    } else {
        root.join(requested_path)
    };

    // Try to canonicalize (resolves symlinks). If the path does not yet exist,
    // fall back to lexical analysis.
    let resolved = if candidate.exists() {
        candidate.canonicalize().map_err(|e| SkillError::Runtime {
            runtime: "vfs".to_owned(),
            message: format!("failed to canonicalize path: {e}"),
        })?
    } else {
        // Lexical check: ensure no `..` components remain after normalisation.
        lexical_normalize(&candidate)
    };

    let canonical_root = if root.exists() {
        root.canonicalize().map_err(|e| SkillError::Runtime {
            runtime: "vfs".to_owned(),
            message: format!("failed to canonicalize project root: {e}"),
        })?
    } else {
        lexical_normalize(root)
    };

    if !resolved.starts_with(&canonical_root) {
        return Err(SkillError::Runtime {
            runtime: "vfs".to_owned(),
            message: format!(
                "path '{}' escapes project root '{}'",
                requested,
                canonical_root.display()
            ),
        });
    }

    Ok(resolved)
}

/// Simple lexical path normalisation that collapses `.` and `..` without
/// touching the filesystem.
#[allow(dead_code)]
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // Only pop if the last item is a normal component.
                if components
                    .last()
                    .is_some_and(|c| matches!(c, std::path::Component::Normal(_)))
                {
                    let _ = components.pop();
                } else {
                    components.push(component);
                }
            }
            std::path::Component::CurDir => {
                // Skip `.` components entirely.
            }
            _ => {
                components.push(component);
            }
        }
    }
    components.iter().collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn simple_relative_path() {
        let root = Path::new("/tmp/project");
        let result = safe_resolve(root, "src/main.rs").expect("should succeed");
        assert!(result.starts_with("/tmp/project"));
        assert!(result.ends_with("src/main.rs"));
    }

    #[test]
    fn absolute_path_is_rebased() {
        let root = Path::new("/tmp/project");
        let result = safe_resolve(root, "/src/main.rs").expect("should succeed");
        assert!(result.starts_with("/tmp/project"));
    }

    #[test]
    fn parent_traversal_rejected() {
        let root = Path::new("/tmp/project");
        let err = safe_resolve(root, "../etc/passwd").expect_err("should reject traversal");
        assert!(matches!(err, SkillError::Runtime { .. }));
    }

    #[test]
    fn double_parent_traversal_rejected() {
        let root = Path::new("/tmp/project");
        let err =
            safe_resolve(root, "a/../../etc/passwd").expect_err("should reject deep traversal");
        assert!(matches!(err, SkillError::Runtime { .. }));
    }

    #[test]
    fn dot_component_stripped() {
        let root = Path::new("/tmp/project");
        let result = safe_resolve(root, "./src/./main.rs").expect("should succeed");
        assert!(result.starts_with("/tmp/project"));
    }
}

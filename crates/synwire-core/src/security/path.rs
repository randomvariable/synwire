//! Path traversal protection for tool file operations.

use crate::error::{SynwireError, ToolError};

/// Validate a path for safety (no traversal, no null bytes, no absolute paths).
///
/// This is a defence-in-depth measure to prevent tools from accessing files
/// outside their intended sandbox.
///
/// # Errors
///
/// Returns [`SynwireError::Tool`] with [`ToolError::PathTraversal`] if the path:
/// - Contains null bytes
/// - Contains `..` path traversal components
/// - Starts with `/` or `\` (absolute path)
/// - Contains Windows-style drive letters (e.g. `C:`)
pub fn validate_tool_path(path: &str) -> Result<(), SynwireError> {
    if path.contains('\0') {
        return Err(SynwireError::Tool(ToolError::PathTraversal {
            path: path.into(),
        }));
    }
    if path.contains("..") {
        return Err(SynwireError::Tool(ToolError::PathTraversal {
            path: path.into(),
        }));
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(SynwireError::Tool(ToolError::PathTraversal {
            path: path.into(),
        }));
    }
    // Reject Windows drive letters like C: or D:
    if path.len() >= 2 && path.as_bytes()[0].is_ascii_alphabetic() && path.as_bytes()[1] == b':' {
        return Err(SynwireError::Tool(ToolError::PathTraversal {
            path: path.into(),
        }));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn allows_safe_relative_paths() {
        validate_tool_path("foo/bar.txt").unwrap();
        validate_tool_path("data").unwrap();
        validate_tool_path("a/b/c/d.json").unwrap();
    }

    #[test]
    fn rejects_null_bytes() {
        let err = validate_tool_path("foo\0bar").unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn rejects_dot_dot() {
        let err = validate_tool_path("../etc/passwd").unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn rejects_embedded_dot_dot() {
        let err = validate_tool_path("foo/../../bar").unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn rejects_absolute_unix() {
        let err = validate_tool_path("/etc/passwd").unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn rejects_absolute_windows() {
        let err = validate_tool_path("\\Windows\\System32").unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn rejects_drive_letter() {
        let err = validate_tool_path("C:\\Windows").unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }
}

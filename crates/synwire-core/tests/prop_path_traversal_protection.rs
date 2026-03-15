//! Property test: Path traversal protection rejects all dangerous paths.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_core::security::validate_tool_path;

proptest! {
    /// Any path containing `..` should be rejected.
    #[test]
    fn rejects_dot_dot_anywhere(
        prefix in "[a-z/]{0,10}",
        suffix in "[a-z/]{0,10}",
    ) {
        let path = format!("{prefix}..{suffix}");
        assert!(
            validate_tool_path(&path).is_err(),
            "path containing '..' should be rejected: {path}"
        );
    }

    /// Any path starting with `/` should be rejected.
    #[test]
    fn rejects_absolute_unix_paths(suffix in "[a-z/]{1,20}") {
        let path = format!("/{suffix}");
        assert!(
            validate_tool_path(&path).is_err(),
            "absolute path should be rejected: {path}"
        );
    }

    /// Any path starting with `\` should be rejected.
    #[test]
    fn rejects_absolute_windows_paths(suffix in "[a-z\\\\]{1,20}") {
        let path = format!("\\{suffix}");
        assert!(
            validate_tool_path(&path).is_err(),
            "backslash-prefixed path should be rejected: {path}"
        );
    }

    /// Paths with null bytes should always be rejected.
    #[test]
    fn rejects_null_bytes(
        prefix in "[a-z]{0,10}",
        suffix in "[a-z]{0,10}",
    ) {
        let path = format!("{prefix}\0{suffix}");
        assert!(
            validate_tool_path(&path).is_err(),
            "null byte path should be rejected"
        );
    }

    /// Windows drive letters should be rejected.
    #[test]
    fn rejects_drive_letters(drive in "[A-Za-z]", suffix in "[a-z/]{0,20}") {
        let path = format!("{drive}:{suffix}");
        assert!(
            validate_tool_path(&path).is_err(),
            "drive letter path should be rejected: {path}"
        );
    }

    /// Safe relative paths should be accepted.
    #[test]
    fn accepts_safe_relative_paths(path in "[a-z][a-z0-9_/]{0,30}") {
        // Filter out paths that happen to contain `..` by chance.
        prop_assume!(!path.contains(".."));
        assert!(
            validate_tool_path(&path).is_ok(),
            "safe relative path should be accepted: {path}"
        );
    }
}
